//! Minimal reader/writer for NumPy `.npy` files (format version 1.0), little-endian,
//! C order, dtypes `<f4` and `<f8`.

use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;
use thiserror::Error;

/// Errors from reading or writing .npy files.
#[derive(Debug, Error)]
pub enum NpyError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("not a .npy file or unsupported format: {0}")]
    Format(String),
    #[error("shape {shape:?} does not match data length {len}")]
    Shape { shape: Vec<usize>, len: usize },
}

/// Element types supported in .npy files.
pub trait NpyElement: Copy + Sized {
    /// NumPy dtype descriptor, e.g. `<f8`.
    const DESCR: &'static str;
    /// Byte width.
    const SIZE: usize;
    fn write_le(self, out: &mut Vec<u8>);
    fn read_le(bytes: &[u8]) -> Self;
}

impl NpyElement for f32 {
    const DESCR: &'static str = "<f4";
    const SIZE: usize = 4;
    fn write_le(self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_le_bytes());
    }
    fn read_le(b: &[u8]) -> Self {
        f32::from_le_bytes([b[0], b[1], b[2], b[3]])
    }
}

impl NpyElement for f64 {
    const DESCR: &'static str = "<f8";
    const SIZE: usize = 8;
    fn write_le(self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_le_bytes());
    }
    fn read_le(b: &[u8]) -> Self {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[..8]);
        f64::from_le_bytes(a)
    }
}

fn header(descr: &str, shape: &[usize]) -> Vec<u8> {
    let shape_str = match shape.len() {
        0 => "()".to_string(),
        1 => format!("({},)", shape[0]),
        _ => format!(
            "({})",
            shape
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    };
    let dict = format!("{{'descr': '{descr}', 'fortran_order': False, 'shape': {shape_str}, }}");
    // magic(6) + version(2) + header_len(2) + dict + padding + '\n' must be a multiple of 64.
    let unpadded = 10 + dict.len() + 1;
    let pad = (64 - unpadded % 64) % 64;
    let mut h = Vec::with_capacity(unpadded + pad);
    h.extend_from_slice(b"\x93NUMPY\x01\x00");
    let hlen = (dict.len() + pad + 1) as u16;
    h.extend_from_slice(&hlen.to_le_bytes());
    h.extend_from_slice(dict.as_bytes());
    h.extend(std::iter::repeat_n(b' ', pad));
    h.push(b'\n');
    h
}

/// Encode an array as .npy bytes.
pub fn to_bytes<T: NpyElement>(data: &[T], shape: &[usize]) -> Result<Vec<u8>, NpyError> {
    let n: usize = shape.iter().product();
    if n != data.len() {
        return Err(NpyError::Shape {
            shape: shape.to_vec(),
            len: data.len(),
        });
    }
    let mut out = header(T::DESCR, shape);
    out.reserve(n * T::SIZE);
    for &x in data {
        x.write_le(&mut out);
    }
    Ok(out)
}

/// Write an array to a .npy file.
pub fn write<T: NpyElement>(path: &Path, data: &[T], shape: &[usize]) -> Result<(), NpyError> {
    let bytes = to_bytes(data, shape)?;
    let mut f = BufWriter::new(File::create(path)?);
    f.write_all(&bytes)?;
    f.flush()?;
    Ok(())
}

fn dict_value<'a>(dict: &'a str, key: &str) -> Result<&'a str, NpyError> {
    let pat = format!("'{key}':");
    let start = dict
        .find(&pat)
        .ok_or_else(|| NpyError::Format(format!("missing key {key}")))?
        + pat.len();
    Ok(dict[start..].trim_start())
}

/// Decode .npy bytes into data and shape.
pub fn from_bytes<T: NpyElement>(bytes: &[u8]) -> Result<(Vec<T>, Vec<usize>), NpyError> {
    if bytes.len() < 10 || &bytes[..6] != b"\x93NUMPY" {
        return Err(NpyError::Format("bad magic".into()));
    }
    let (hlen, hstart) = match bytes[6] {
        1 => (u16::from_le_bytes([bytes[8], bytes[9]]) as usize, 10),
        2 | 3 => {
            if bytes.len() < 12 {
                return Err(NpyError::Format("truncated header".into()));
            }
            (
                u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize,
                12,
            )
        }
        v => return Err(NpyError::Format(format!("version {v}"))),
    };
    let dict = std::str::from_utf8(
        bytes
            .get(hstart..hstart + hlen)
            .ok_or_else(|| NpyError::Format("truncated header".into()))?,
    )
    .map_err(|e| NpyError::Format(e.to_string()))?;

    let descr = dict_value(dict, "descr")?;
    let expected = format!("'{}'", T::DESCR);
    if !descr.starts_with(&expected) {
        return Err(NpyError::Format(format!(
            "dtype {descr}, expected {}",
            T::DESCR
        )));
    }
    if dict_value(dict, "fortran_order")?.starts_with("True") {
        return Err(NpyError::Format(
            "fortran_order arrays are not supported".into(),
        ));
    }
    let s = dict_value(dict, "shape")?;
    let close = s
        .find(')')
        .ok_or_else(|| NpyError::Format("bad shape".into()))?;
    let shape: Vec<usize> = s[1..close]
        .split(',')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(|t| {
            t.parse()
                .map_err(|_| NpyError::Format(format!("bad shape entry {t}")))
        })
        .collect::<Result<_, _>>()?;
    let n: usize = shape.iter().product();
    let body = &bytes[hstart + hlen..];
    if body.len() != n * T::SIZE {
        return Err(NpyError::Shape {
            shape,
            len: body.len() / T::SIZE,
        });
    }
    let data = body.chunks_exact(T::SIZE).map(T::read_le).collect();
    Ok((data, shape))
}

/// Read a .npy file into data and shape.
pub fn read<T: NpyElement>(path: &Path) -> Result<(Vec<T>, Vec<usize>), NpyError> {
    let mut bytes = Vec::new();
    BufReader::new(File::open(path)?).read_to_end(&mut bytes)?;
    from_bytes(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_is_aligned() {
        for shape in [vec![], vec![7], vec![3, 4], vec![20, 64, 128]] {
            let h = header("<f8", &shape);
            assert_eq!(h.len() % 64, 0);
            assert_eq!(*h.last().unwrap(), b'\n');
        }
    }

    #[test]
    fn round_trip_f64_and_f32() {
        let data: Vec<f64> = (0..24).map(|i| (i as f64).sin() * 1e3).collect();
        let b = to_bytes(&data, &[2, 3, 4]).unwrap();
        let (d2, s2) = from_bytes::<f64>(&b).unwrap();
        assert_eq!(d2, data);
        assert_eq!(s2, vec![2, 3, 4]);

        let f: Vec<f32> = data.iter().map(|&x| x as f32).collect();
        let dir = std::env::temp_dir().join(format!("toyclim_npy_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("a.npy");
        write(&p, &f, &[6, 4]).unwrap();
        let (f2, s) = read::<f32>(&p).unwrap();
        assert_eq!(f2, f);
        assert_eq!(s, vec![6, 4]);
        assert!(read::<f64>(&p).is_err(), "dtype mismatch must be rejected");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn shape_mismatch_rejected() {
        assert!(to_bytes(&[1.0f64, 2.0], &[3]).is_err());
    }
}
