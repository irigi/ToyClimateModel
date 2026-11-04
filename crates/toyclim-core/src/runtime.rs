//! Running a model on a worker thread for interactive viewers (SPEC.md §7).
//!
//! The [`ModelRunner`] owns the model on its own thread. The UI sends [`Command`]s and
//! reads the latest [`Snapshot`] through a double buffer (`Mutex<Arc<Snapshot>>` swap);
//! reading never waits for a model step. This module has no Godot dependency: the gdext
//! wrapper in `toyclim-godot` (or a file bridge) is a thin layer on top.

use crate::grid::LatLonRegridder;
use crate::model::Model;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Commands from the UI to the worker.
#[derive(Debug, Clone)]
pub enum Command {
    /// Start or resume continuous stepping.
    Run,
    /// Stop stepping (the state is kept).
    Pause,
    /// Pacing while running: target model steps per wall second; 0 = as fast as possible.
    SetStepsPerSecond(f64),
    /// Take exactly n steps, then pause.
    Step(u64),
    /// Which fields the snapshots carry (regridded to the equirectangular image grid).
    SetFields(Vec<String>),
    /// Minimum wall time between snapshots while running \[s\] (default 1/30).
    SetPublishInterval(f64),
    /// Write a restart directory (both leapfrog levels) at the current step.
    SaveRestart(PathBuf),
    /// End the worker thread.
    Shutdown,
}

/// One field in a snapshot: equirectangular `[lev][row][col]` f32, row 0 = 90°N edge
/// cell, column 0 = 0°E, linear in latitude and longitude.
#[derive(Debug, Clone)]
pub struct SnapshotField {
    pub name: String,
    pub nlev: usize,
    pub width: usize,
    pub height: usize,
    pub data: Vec<f32>,
    /// Min and max per level (for colour scaling).
    pub range: Vec<(f32, f32)>,
}

impl SnapshotField {
    /// Data of one level, `[row][col]`.
    pub fn level(&self, lev: usize) -> &[f32] {
        let n = self.width * self.height;
        &self.data[lev * n..(lev + 1) * n]
    }
}

/// A consistent view of the model at one step.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub step: u64,
    /// Model time \[s\].
    pub time: f64,
    pub running: bool,
    pub steps_per_second_target: f64,
    /// Measured model steps per wall second over the last publish interval.
    pub steps_per_second_actual: f64,
    pub diagnostics: Vec<(String, f64)>,
    pub fields: Vec<SnapshotField>,
    /// All field names the model offers.
    pub available_fields: Vec<String>,
    /// σ levels (empty for 2D models).
    pub sigma: Vec<f64>,
    /// Set if the model stopped because of an error (e.g. non-finite state).
    pub error: Option<String>,
    /// Result of the last SaveRestart.
    pub last_save: Option<Result<PathBuf, String>>,
}

/// Owns a model on a worker thread.
pub struct ModelRunner {
    tx: Sender<Command>,
    latest: Arc<Mutex<Arc<Snapshot>>>,
    handle: Option<JoinHandle<()>>,
}

struct Worker {
    model: Box<dyn Model>,
    config_json: serde_json::Value,
    regrid: LatLonRegridder,
    fields: Vec<String>,
    running: bool,
    pending_steps: u64,
    sps_target: f64,
    publish_interval: f64,
    error: Option<String>,
    last_save: Option<Result<PathBuf, String>>,
    latest: Arc<Mutex<Arc<Snapshot>>>,
}

impl ModelRunner {
    /// Start the worker. `config_json` is stored in restart files. `image_size` is the
    /// (width, height) of snapshot fields; None → 2·N_lon × N_lon.
    pub fn start(
        model: Box<dyn Model>,
        config_json: serde_json::Value,
        image_size: Option<(usize, usize)>,
    ) -> Self {
        let regrid = match image_size {
            Some((w, h)) => LatLonRegridder::new(model.grid(), w, h),
            None => LatLonRegridder::default_for(model.grid()),
        };
        let latest = Arc::new(Mutex::new(Arc::new(Snapshot::default())));
        let (tx, rx) = channel();
        let mut w = Worker {
            fields: model
                .field_names()
                .iter()
                .take(1)
                .map(|s| s.to_string())
                .collect(),
            model,
            config_json,
            regrid,
            running: false,
            pending_steps: 0,
            sps_target: 0.0,
            publish_interval: 1.0 / 30.0,
            error: None,
            last_save: None,
            latest: latest.clone(),
        };
        w.publish(0.0);
        let handle = std::thread::Builder::new()
            .name("toyclim-model".into())
            .spawn(move || w.run(rx))
            .expect("spawn model thread");
        ModelRunner {
            tx,
            latest,
            handle: Some(handle),
        }
    }

    /// Send a command (never blocks).
    pub fn send(&self, cmd: Command) {
        let _ = self.tx.send(cmd);
    }

    /// The latest published snapshot (cheap: clones an Arc under a short lock).
    pub fn snapshot(&self) -> Arc<Snapshot> {
        self.latest.lock().map(|g| g.clone()).unwrap_or_default()
    }
}

impl Drop for ModelRunner {
    fn drop(&mut self) {
        let _ = self.tx.send(Command::Shutdown);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

impl Worker {
    fn run(&mut self, rx: Receiver<Command>) {
        let mut last_publish = Instant::now();
        let mut steps_since = 0u64;
        let mut next_step_at = Instant::now();
        loop {
            let active = (self.running || self.pending_steps > 0) && self.error.is_none();
            // Wait for commands: block when idle, otherwise only until the next step is due.
            let cmd = if active {
                let wait = next_step_at.saturating_duration_since(Instant::now());
                match rx.recv_timeout(wait) {
                    Ok(c) => Some(c),
                    Err(RecvTimeoutError::Timeout) => None,
                    Err(RecvTimeoutError::Disconnected) => return,
                }
            } else {
                match rx.recv() {
                    Ok(c) => Some(c),
                    Err(_) => return,
                }
            };
            if let Some(c) = cmd {
                if !self.handle(c) {
                    return;
                }
                // Drain any further queued commands before stepping.
                while let Ok(c) = rx.try_recv() {
                    if !self.handle(c) {
                        return;
                    }
                }
                self.publish(0.0);
                continue;
            }
            // Step.
            self.model.step();
            steps_since += 1;
            if self.pending_steps > 0 {
                self.pending_steps -= 1;
            }
            if !self.model.is_finite() {
                self.error = Some(format!(
                    "non-finite state at step {} (t = {:.3} days)",
                    self.model.step_count(),
                    self.model.time() / 86400.0
                ));
                self.running = false;
                self.pending_steps = 0;
            }
            let now = Instant::now();
            next_step_at = if self.sps_target > 0.0 && self.running {
                next_step_at.max(now - Duration::from_millis(250))
                    + Duration::from_secs_f64(1.0 / self.sps_target)
            } else {
                now
            };
            let since = now.duration_since(last_publish).as_secs_f64();
            let done_stepping = self.pending_steps == 0 && !self.running;
            if since >= self.publish_interval || done_stepping || self.error.is_some() {
                self.publish(steps_since as f64 / since.max(1e-9));
                last_publish = now;
                steps_since = 0;
            }
        }
    }

    /// Apply a command; false means shut down.
    fn handle(&mut self, c: Command) -> bool {
        match c {
            Command::Run => self.running = true,
            Command::Pause => {
                self.running = false;
                self.pending_steps = 0;
            }
            Command::SetStepsPerSecond(s) => self.sps_target = s.max(0.0),
            Command::Step(n) => {
                self.running = false;
                self.pending_steps = n;
            }
            Command::SetFields(f) => self.fields = f,
            Command::SetPublishInterval(s) => self.publish_interval = s.max(0.0),
            Command::SaveRestart(dir) => {
                let (arrays, scalars) = self.model.restart_state();
                self.last_save = Some(
                    crate::io::restart::write(&dir, &arrays, &scalars, self.config_json.clone())
                        .map(|_| dir)
                        .map_err(|e| e.to_string()),
                );
            }
            Command::Shutdown => return false,
        }
        true
    }

    fn publish(&self, sps: f64) {
        let g = self.model.grid();
        let npts = g.nlat * g.nlon;
        let mut fields = Vec::new();
        for name in &self.fields {
            if let Some(f) = self.model.field(name) {
                let mut data = Vec::with_capacity(f.nlev * self.regrid.width * self.regrid.height);
                let mut range = Vec::with_capacity(f.nlev);
                for lev in 0..f.nlev {
                    let img = self.regrid.regrid(&f.data[lev * npts..(lev + 1) * npts]);
                    let (lo, hi) = img
                        .iter()
                        .fold((f32::INFINITY, f32::NEG_INFINITY), |(a, b), &x| {
                            (a.min(x), b.max(x))
                        });
                    range.push((lo, hi));
                    data.extend(img);
                }
                fields.push(SnapshotField {
                    name: name.clone(),
                    nlev: f.nlev,
                    width: self.regrid.width,
                    height: self.regrid.height,
                    data,
                    range,
                });
            }
        }
        let snap = Snapshot {
            step: self.model.step_count(),
            time: self.model.time(),
            running: self.running,
            steps_per_second_target: self.sps_target,
            steps_per_second_actual: sps,
            diagnostics: self
                .model
                .diagnostics()
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
            fields,
            available_fields: self
                .model
                .field_names()
                .iter()
                .map(|s| s.to_string())
                .collect(),
            sigma: self.model.sigma(),
            error: self.error.clone(),
            last_save: self.last_save.clone(),
        };
        if let Ok(mut g) = self.latest.lock() {
            *g = Arc::new(snap);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn model() -> Box<dyn Model> {
        let cfg = Config::parse(
            "model = \"shallow_water\"\ntruncation = 21\ndt = \"30min\"\nlength = \"1day\"\n\
             output_every = \"1day\"\ninitial_condition = { case = \"williamson6\" }\n",
        )
        .unwrap();
        crate::build::build_model(&cfg).unwrap()
    }

    fn wait_for(r: &ModelRunner, pred: impl Fn(&Snapshot) -> bool) -> Arc<Snapshot> {
        let t0 = Instant::now();
        loop {
            let s = r.snapshot();
            if pred(&s) {
                return s;
            }
            assert!(t0.elapsed() < Duration::from_secs(20), "timeout");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn step_run_pause_and_fields() {
        let r = ModelRunner::start(model(), serde_json::Value::Null, Some((64, 32)));
        r.send(Command::SetFields(vec!["h".into(), "u".into()]));
        r.send(Command::Step(3));
        let s = wait_for(&r, |s| s.step == 3 && !s.running);
        assert_eq!(s.fields.len(), 2);
        assert_eq!(s.fields[0].data.len(), 64 * 32);
        assert!(s.fields[0].range[0].0 > 7000.0); // RH depth ≈ 8 km
        r.send(Command::Run);
        wait_for(&r, |s| s.step > 10);
        r.send(Command::Pause);
        let a = wait_for(&r, |s| !s.running);
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(r.snapshot().step, a.step, "paused model must not advance");
    }

    #[test]
    fn pacing_limits_speed() {
        let r = ModelRunner::start(model(), serde_json::Value::Null, Some((16, 8)));
        r.send(Command::SetStepsPerSecond(50.0));
        r.send(Command::Run);
        std::thread::sleep(Duration::from_millis(400));
        let s = r.snapshot();
        assert!(s.step >= 5 && s.step <= 40, "step {}", s.step);
    }
}
