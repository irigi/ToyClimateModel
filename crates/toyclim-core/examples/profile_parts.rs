//! Rough timing of transform components (development aid).
use std::time::Instant;
use toyclim_core::util::SplitMix64;
use toyclim_core::{Complex, Transform};

fn main() {
    let t: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(42);
    let tr = Transform::new(t, 6.37122e6);
    let mut rng = SplitMix64(1);
    let f: Vec<Complex> = (0..tr.layout.len())
        .map(|_| Complex::new(rng.symmetric() as _, rng.symmetric() as _))
        .collect();
    let g = tr.synthesis(&f);
    let n = 2000;
    let time = |name: &str, mut op: Box<dyn FnMut()>| {
        let t0 = Instant::now();
        for _ in 0..n {
            op();
        }
        println!(
            "{name:<20} {:8.1} µs",
            t0.elapsed().as_secs_f64() * 1e6 / n as f64
        );
    };
    time(
        "fourier (fwd fft)",
        Box::new(|| {
            std::hint::black_box(tr.fourier(&g));
        }),
    );
    time(
        "synthesis",
        Box::new(|| {
            std::hint::black_box(tr.synthesis(&f));
        }),
    );
    time(
        "analysis",
        Box::new(|| {
            std::hint::black_box(tr.analysis(&g));
        }),
    );
    time(
        "uv",
        Box::new(|| {
            std::hint::black_box(tr.uv_from_psi_chi(&f, &f));
        }),
    );
    time(
        "div_curl",
        Box::new(|| {
            std::hint::black_box(tr.div_curl(&g, &g));
        }),
    );
}
