//! Online climatology: time means accumulated during a run (after a spin-up), so long
//! runs need not store daily 3D output.
//!
//! Zonal statistics per level and latitude (`[lev][lat]`), from samples every
//! `every` seconds after `start`:
//! - means [u], [v], [t] and second moments [uu], [vv], [tt], [uv], [vt], where [·] is the
//!   zonal mean at each sample;
//! - eddy covariances [x*y*] = [xy] − [x][y] per sample, accumulated as `eddy_*`
//!   (transient + stationary eddies relative to the instantaneous zonal mean).
//!
//! Also full time-mean maps of selected fields (`mean_<field>`). Sums are kept in f64.

use crate::model::Model;

/// Zonal-statistics names (each `[lev][lat]`).
pub const ZONAL: &[&str] = &[
    "u", "v", "t", "uu", "vv", "tt", "uv", "vt", "eddy_uu", "eddy_vv", "eddy_tt", "eddy_uv",
    "eddy_vt",
];

/// Accumulates statistics.
pub struct Climatology {
    pub start: f64,
    pub every_steps: u64,
    pub count: u64,
    pub nlev: usize,
    pub nlat: usize,
    /// Zonal sums, in the order of [`ZONAL`].
    pub zonal: Vec<Vec<f64>>,
    /// Time-mean maps: (field name, nlev, sums).
    pub maps: Vec<(String, usize, Vec<f64>)>,
    /// Zonal mean of surface pressure.
    pub ps: Vec<f64>,
    /// Per-calendar-month map sums `[field][month][...]` and sample counts per month
    /// (empty unless monthly statistics are enabled).
    pub monthly: Vec<Vec<Vec<f64>>>,
    pub monthly_count: Vec<u64>,
    /// Calendar used for the monthly binning.
    pub calendar: Option<crate::orbit::Calendar>,
}

impl Climatology {
    /// `map_fields` must exist in the model; zonal statistics need fields u, v, t.
    pub fn new(
        model: &dyn Model,
        start: f64,
        every_steps: u64,
        map_fields: &[String],
    ) -> Result<Self, String> {
        let g = model.grid();
        let names = model.field_names();
        for f in ["u", "v", "t", "ps"] {
            if !names.contains(&f) {
                return Err(format!(
                    "climatology needs field {f:?} (primitive-equation model)"
                ));
            }
        }
        let nlev = model.field("u").map(|f| f.nlev).unwrap_or(1);
        let mut maps = Vec::new();
        for f in map_fields {
            let fld = model
                .field(f)
                .ok_or_else(|| format!("climatology map field {f:?} unknown"))?;
            maps.push((f.clone(), fld.nlev, vec![0.0; fld.nlev * g.nlat * g.nlon]));
        }
        Ok(Climatology {
            start,
            every_steps: every_steps.max(1),
            count: 0,
            nlev,
            nlat: g.nlat,
            zonal: vec![vec![0.0; nlev * g.nlat]; ZONAL.len()],
            maps,
            ps: vec![0.0; g.nlat],
            monthly: Vec::new(),
            monthly_count: Vec::new(),
            calendar: None,
        })
    }

    /// Also accumulate the map fields per calendar month.
    pub fn enable_monthly(&mut self, calendar: crate::orbit::Calendar) {
        let nm = calendar.months.len();
        self.monthly = self
            .maps
            .iter()
            .map(|(_, _, s)| vec![vec![0.0; s.len()]; nm])
            .collect();
        self.monthly_count = vec![0; nm];
        self.calendar = Some(calendar);
    }

    /// Sample if due (time ≥ start and step multiple of `every_steps`).
    pub fn maybe_sample(&mut self, model: &dyn Model) {
        if model.time() + 1e-6 < self.start || !model.step_count().is_multiple_of(self.every_steps)
        {
            return;
        }
        self.sample(model);
    }

    fn sample(&mut self, model: &dyn Model) {
        let g = model.grid();
        let (nlat, nlon) = (g.nlat, g.nlon);
        let npts = nlat * nlon;
        let u = model.field("u").unwrap().data;
        let v = model.field("v").unwrap().data;
        let t = model.field("t").unwrap().data;
        let ps = model.field("ps").unwrap().data;
        let inv = 1.0 / nlon as f64;
        for k in 0..self.nlev {
            for j in 0..nlat {
                let o = k * npts + j * nlon;
                let row = |x: &[crate::Real]| -> Vec<f64> {
                    x[o..o + nlon].iter().map(|&a| a as f64).collect()
                };
                let (ur, vr, tr) = (row(&u), row(&v), row(&t));
                let mean = |x: &[f64]| x.iter().sum::<f64>() * inv;
                let dot =
                    |x: &[f64], y: &[f64]| x.iter().zip(y).map(|(a, b)| a * b).sum::<f64>() * inv;
                let (mu, mv, mt) = (mean(&ur), mean(&vr), mean(&tr));
                let (uu, vv, tt, uv, vt) = (
                    dot(&ur, &ur),
                    dot(&vr, &vr),
                    dot(&tr, &tr),
                    dot(&ur, &vr),
                    dot(&vr, &tr),
                );
                let vals = [
                    mu,
                    mv,
                    mt,
                    uu,
                    vv,
                    tt,
                    uv,
                    vt,
                    uu - mu * mu,
                    vv - mv * mv,
                    tt - mt * mt,
                    uv - mu * mv,
                    vt - mv * mt,
                ];
                let idx = k * nlat + j;
                for (s, x) in self.zonal.iter_mut().zip(vals) {
                    s[idx] += x;
                }
            }
        }
        for j in 0..nlat {
            self.ps[j] += ps[j * nlon..(j + 1) * nlon]
                .iter()
                .map(|&a| a as f64)
                .sum::<f64>()
                * inv;
        }
        let month = self.calendar.as_ref().map(|c| c.month(model.time()));
        for (fi, (name, _, sums)) in self.maps.iter_mut().enumerate() {
            let f = model.field(name).unwrap().data;
            for (s, x) in sums.iter_mut().zip(&f) {
                *s += *x as f64;
            }
            if let Some(m) = month {
                for (s, x) in self.monthly[fi][m].iter_mut().zip(&f) {
                    *s += *x as f64;
                }
            }
        }
        if let Some(m) = month {
            self.monthly_count[m] += 1;
        }
        self.count += 1;
    }
}
