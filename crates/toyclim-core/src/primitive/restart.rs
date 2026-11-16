//! Restart state of the primitive-equation model: both leapfrog levels (spectral, per
//! level), the surface, and scalar counters.

use super::state::PeState;
use super::Primitive;
use crate::model::RestartArray;
use crate::spectral::Complex;
use crate::Real;

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
                for (it, gq) in s.gq.iter().enumerate() {
                    a.push((format!("{lvl}_gtracer{it}_{k:03}"), pack(&gq[k])));
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
                ("budget_fixer".to_string(), b.fixer),
                ("budget_runoff".to_string(), b.runoff),
                ("budget_ocean_uptake".to_string(), b.ocean_uptake),
                ("budget_ice_cap".to_string(), b.ice_cap),
                ("budget_qflux".to_string(), b.qflux),
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
            s.gq = self.curr.gq.clone();
            for k in 0..self.nlev() {
                s.vor[k].copy_from_slice(get(&format!("{lvl}_vor_{k:03}"))?);
                s.div[k].copy_from_slice(get(&format!("{lvl}_div_{k:03}"))?);
                s.tmp[k].copy_from_slice(get(&format!("{lvl}_tmp_{k:03}"))?);
                for it in 0..ntr {
                    s.tracers[it][k].copy_from_slice(get(&format!("{lvl}_tracer{it}_{k:03}"))?);
                }
                for (it, gq) in s.gq.iter_mut().enumerate() {
                    let name = format!("{lvl}_gtracer{it}_{k:03}");
                    let a = arrays
                        .iter()
                        .find(|(n, _)| *n == name)
                        .ok_or_else(|| format!("restart lacks {name}"))?;
                    if a.1.len() != gq[k].len().div_ceil(2) {
                        return Err(format!("{name}: wrong grid"));
                    }
                    unpack(&a.1, &mut gq[k]);
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
                "budget_fixer" => self.budget.fixer = *v,
                "budget_runoff" => self.budget.runoff = *v,
                "budget_ocean_uptake" => self.budget.ocean_uptake = *v,
                "budget_ice_cap" => self.budget.ice_cap = *v,
                "budget_qflux" => self.budget.qflux = *v,
                _ => {}
            }
        }
        Ok(())
    }
}

/// Grid values two per complex number (restart arrays are complex).
fn pack(x: &[Real]) -> Vec<Complex> {
    x.chunks(2)
        .map(|c| Complex::new(c[0], c.get(1).copied().unwrap_or(0.0)))
        .collect()
}

fn unpack(packed: &[Complex], out: &mut [Real]) {
    for (i, c) in packed.iter().enumerate() {
        out[2 * i] = c.re;
        if 2 * i + 1 < out.len() {
            out[2 * i + 1] = c.im;
        }
    }
}
