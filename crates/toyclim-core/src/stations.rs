//! Station time series: model fields sampled at fixed points (bilinear interpolation).

use crate::config::StationConfig;
use crate::grid::PointSampler;
use crate::model::Model;

/// Records the configured fields at the configured stations.
pub struct StationRecorder {
    pub names: Vec<String>,
    pub lat_deg: Vec<f64>,
    pub lon_deg: Vec<f64>,
    pub fields: Vec<String>,
    samplers: Vec<PointSampler>,
    /// Sample times \[s\].
    pub times: Vec<f64>,
    /// Per field: levels per sample and the samples `[time][station][lev]`.
    pub data: Vec<(usize, Vec<f32>)>,
}

impl StationRecorder {
    /// `fields` empty → every field the model offers.
    pub fn new(model: &dyn Model, stations: &[StationConfig], fields: &[String]) -> Self {
        let grid = model.grid();
        let fields: Vec<String> = if fields.is_empty() {
            model.field_names().iter().map(|s| s.to_string()).collect()
        } else {
            fields.to_vec()
        };
        StationRecorder {
            names: stations.iter().map(|s| s.name.clone()).collect(),
            lat_deg: stations.iter().map(|s| s.lat).collect(),
            lon_deg: stations.iter().map(|s| s.lon).collect(),
            samplers: stations
                .iter()
                .map(|s| PointSampler::new(grid, s.lat.to_radians(), s.lon.to_radians()))
                .collect(),
            data: vec![(0, Vec::new()); fields.len()],
            fields,
            times: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.samplers.is_empty()
    }

    /// Sample the model's current state. Unknown fields are an error.
    pub fn record(&mut self, model: &dyn Model) -> Result<(), String> {
        if self.is_empty() {
            return Ok(());
        }
        let g = model.grid();
        let npts = g.nlat * g.nlon;
        for (fi, name) in self.fields.iter().enumerate() {
            let f = model
                .field(name)
                .ok_or_else(|| format!("station field {name:?} not provided by the model"))?;
            let (nlev, buf) = &mut self.data[fi];
            *nlev = f.nlev;
            for s in &self.samplers {
                for lev in 0..f.nlev {
                    let slice = &f.data[lev * npts..(lev + 1) * npts];
                    buf.push(s.sample(slice, g.nlon) as f32);
                }
            }
        }
        self.times.push(model.time());
        Ok(())
    }
}
