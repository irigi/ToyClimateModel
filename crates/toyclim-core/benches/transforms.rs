//! M1 test 7: synthesis and analysis per field at T42 and T85.
#![allow(clippy::unnecessary_cast)]
use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use toyclim_core::util::SplitMix64;
use toyclim_core::{Complex, Transform};

fn bench(c: &mut Criterion) {
    for t in [42usize, 85] {
        let tr = Transform::new(t, 6.37122e6);
        let mut rng = SplitMix64(1);
        let f: Vec<Complex> = (0..tr.layout.len())
            .map(|_| Complex::new(rng.symmetric() as _, rng.symmetric() as _))
            .collect();
        let g = tr.synthesis(&f);
        c.bench_function(&format!("synthesis T{t}"), |b| {
            b.iter(|| tr.synthesis(black_box(&f)))
        });
        c.bench_function(&format!("analysis T{t}"), |b| {
            b.iter(|| tr.analysis(black_box(&g)))
        });
    }
}

fn config() -> Criterion {
    Criterion::default()
        .sample_size(30)
        .measurement_time(std::time::Duration::from_secs(2))
        .warm_up_time(std::time::Duration::from_millis(500))
}

criterion_group! { name = benches; config = config(); targets = bench }
criterion_main!(benches);
