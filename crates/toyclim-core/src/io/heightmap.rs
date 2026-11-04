//! Heightmap input: an equirectangular `.npy` (float32 or float64), `[lat][lon]`, row 0 at
//! the north, metres above sea level (≤ 0 = ocean).

use std::path::Path;

/// Read a heightmap as f32 with its (nlat, nlon).
pub fn read(path: &Path) -> Result<(Vec<f32>, usize, usize), String> {
    let (data, shape) = match super::npy::read::<f32>(path) {
        Ok(x) => x,
        Err(_) => {
            let (d, s) = super::npy::read::<f64>(path).map_err(|e| e.to_string())?;
            (d.iter().map(|&x| x as f32).collect(), s)
        }
    };
    if shape.len() != 2 || shape[0] < 2 || shape[1] < 2 {
        return Err(format!(
            "expected a 2D [lat][lon] array, got shape {shape:?}"
        ));
    }
    if data.iter().any(|x| !x.is_finite()) {
        return Err("heightmap contains non-finite values".into());
    }
    Ok((data, shape[0], shape[1]))
}
