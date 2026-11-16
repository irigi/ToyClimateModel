//! Restart state of the primitive-equation model: both leapfrog levels (spectral, per
//! level), the surface, and scalar counters.

use super::state::PeState;
use super::Primitive;
use crate::model::RestartArray;
use crate::spectral::Complex;

impl Primitive {
    pub(crate) fn save_restart(&self) -> (Vec<RestartArray>, Vec<(String, f64)>) {
        let mut a = Vec::new();
        for (lvl, s) in [("prev", &self.prev), ("curr", &self.curr)] {
            for k in 0..self.nlev() {
                a.push((format!("{lvl}_vor_{k:03}"), s.vor[k].to_vec()));
                a.push((format!("{lvl}_div_{k:03}"), s.div[k].to_vec()));
                a.push((format!("{lvl}_tmp_{k:03}"), s.tmp[k].to_vec()));
                for (it, tq) in s.tracers.iter().enumerate() {
                    a.push((format!("{lvl}_tracer{it}_{k:03}"), tq[k].to_vec()));
                }
            }
            a.push((format!("{lvl}_lnps"), s.lnps.clone()));
        }
        if let Some(sf) = &self.surface {
            a.extend(sf.restart_arrays());
        }
        let mut sc = vec![
            ("steps".to_string(), self.steps as f64),
            ("mass_target".to_string(), self.mass_target),
        ];
        if self.has_budget() {
            let b = &self.budget;
            sc.extend([
                ("budget_toa_net".to_string(), b.toa_net),
                ("budget_sfc_net".to_string(), b.sfc_net),
                ("budget_precip".to_string(), b.precip),
                ("budget_evap".to_string(), b.evap),
                ("budget_diffusion_ke".to_string(), b.diffusion_ke),
            ]);
        }
        (a, sc)
    }

    pub(crate) fn load_restart(
        &mut self,
        arrays: &[RestartArray],
        scalars: &[(String, f64)],
    ) -> Result<(), String> {
        let len = self.tr.layout.len();
        let get = |name: &str| -> Result<&[Complex], String> {
            let a = arrays
                .iter()
                .find(|(n, _)| n == name)
                .ok_or_else(|| format!("restart lacks {name}"))?;
            if a.1.len() != len {
                return Err(format!("{name}: wrong truncation"));
            }
            Ok(&a.1)
        };
        for lvl in ["prev", "curr"] {
            let ntr = self.curr.tracers.len();
            let mut s = PeState::zeros_with_tracers(self.nlev(), len, ntr);
            for k in 0..self.nlev() {
                s.vor[k].copy_from_slice(get(&format!("{lvl}_vor_{k:03}"))?);
                s.div[k].copy_from_slice(get(&format!("{lvl}_div_{k:03}"))?);
                s.tmp[k].copy_from_slice(get(&format!("{lvl}_tmp_{k:03}"))?);
                for it in 0..ntr {
                    s.tracers[it][k].copy_from_slice(get(&format!("{lvl}_tracer{it}_{k:03}"))?);
                }
            }
            s.lnps = get(&format!("{lvl}_lnps"))?.to_vec();
            if lvl == "prev" {
                self.prev = s;
            } else {
                self.curr = s;
            }
        }
        if let Some(sf) = self.surface.as_mut() {
            sf.load_restart(arrays)?;
        }
        for (k, v) in scalars {
            match k.as_str() {
                "steps" => self.steps = *v as u64,
                "mass_target" => self.mass_target = *v,
                "budget_toa_net" => self.budget.toa_net = *v,
                "budget_sfc_net" => self.budget.sfc_net = *v,
                "budget_precip" => self.budget.precip = *v,
                "budget_evap" => self.budget.evap = *v,
                "budget_diffusion_ke" => self.budget.diffusion_ke = *v,
                _ => {}
            }
        }
        Ok(())
    }
}
