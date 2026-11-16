//! Output of the primitive-equation model: named grid fields and global diagnostics.

use super::levels::PRESSURE_FIELDS;
use super::state::grids;
use super::Primitive;
use crate::model::GridField;
use crate::Real;

impl Primitive {
    /// Mass \[kg\], total energy ∫(c_p T + Φ_s + E) dm \[J\], axial angular momentum
    /// \[kg m² s⁻¹\], max |u| \[m/s\] and kinetic energy ∫E dm \[J\].
    pub fn conservation(&self) -> [f64; 5] {
        let tr = &self.tr;
        let g = grids(tr, &self.curr);
        let n = tr.ngrid();
        let nlon = tr.nlon();
        let grav = self.planet.gravity;
        let a = self.planet.radius;
        let om = self.planet.rotation_rate;
        let cp = self.planet.cp;
        let tref = self.planet.t_ref;
        let (mut mass, mut en, mut am) = (
            vec![0.0 as Real; n],
            vec![0.0 as Real; n],
            vec![0.0 as Real; n],
        );
        let mut kin_e = vec![0.0 as Real; n];
        let mut umax = 0.0f64;
        for j in 0..tr.nlat() {
            let c2 = 1.0 - tr.grid.mu[j] * tr.grid.mu[j];
            for p in j * nlon..(j + 1) * nlon {
                let ps = (g.lnps[p] as f64).exp();
                let dm = ps / grav;
                let (mut e, mut m, mut ke) = (self.phis_grid[p] as f64, 0.0, 0.0);
                for k in 0..self.nlev() {
                    let ds = self.levels.dsigma[k];
                    let (u, v) = (g.u[k][p] as f64, g.v[k][p] as f64);
                    let kin = 0.5 * (u * u + v * v) / c2;
                    e += ds * (cp * (g.t[k][p] as f64 + tref) + kin);
                    ke += ds * kin;
                    m += ds * (a * u + om * a * a * c2);
                    umax = umax.max(u.abs() / c2.sqrt());
                }
                mass[p] = dm as Real;
                en[p] = (dm * e) as Real;
                am[p] = (dm * m) as Real;
                kin_e[p] = (dm * ke) as Real;
            }
        }
        [
            tr.integrate(&mass),
            tr.integrate(&en),
            tr.integrate(&am),
            umax,
            tr.integrate(&kin_e),
        ]
    }

    /// Whether the run accumulates energy and water budgets (moist or with a surface).
    pub(crate) fn has_budget(&self) -> bool {
        self.surface.is_some() || self.condensable.is_some()
    }

    /// One slot of the physics exchange as a grid field.
    fn exchange_field(&self, slot: usize) -> Vec<Real> {
        use crate::physics::driver::Exchange as X;
        self.physics
            .exchange
            .chunks(X::N)
            .map(|e| e[slot] as Real)
            .collect()
    }

    /// Column water Σ q Δσ p_s / g of tracer 0 \[kg m⁻²\], if the model is moist.
    fn column_water(&self) -> Option<Vec<Real>> {
        let tq = self.curr.tracers.first()?;
        let tr = &self.tr;
        let ps: Vec<Real> = tr
            .synthesis(&self.curr.lnps)
            .iter()
            .map(|x| x.exp())
            .collect();
        let mut w = vec![0.0 as Real; tr.ngrid()];
        for k in 0..self.nlev() {
            let q = tr.synthesis(&tq[k]);
            let ds = self.levels.dsigma[k] as Real;
            for p in 0..w.len() {
                w[p] += q[p] * ds * ps[p] / self.planet.gravity as Real;
            }
        }
        Some(w)
    }

    pub(crate) fn output_field_names(&self) -> Vec<&'static str> {
        let mut v = vec![
            "u", "v", "t", "vor", "div", "phi", "ps", "phis", "orog", "tas", "uas", "vas",
            "sfcwind",
        ];
        v.extend_from_slice(PRESSURE_FIELDS);
        if !self.curr.tracers.is_empty() {
            v.extend_from_slice(&["q", "rh", "pw"]);
        }
        v.extend_from_slice(&["t_low", "u_low", "v_low", "wind_low", "psl"]);
        if self.surface.is_some() {
            v.extend_from_slice(&["ts", "surface_heat", "land", "bucket"]);
        }
        let reg = &self.physics.registry;
        if !reg.precip_parts().is_empty() {
            v.push("precip");
        }
        v.extend_from_slice(reg.names());
        v
    }

    pub(crate) fn output_field(&self, name: &str) -> Option<GridField> {
        let one = |data: Vec<Real>| Some(GridField { nlev: 1, data });
        let reg = &self.physics.registry;
        match name {
            "ps" => one(self
                .tr
                .synthesis(&self.curr.lnps)
                .iter()
                .map(|x| x.exp())
                .collect()),
            "phis" => one(self.phis_grid.clone()),
            // wind at the lowest model level [m/s] (≈ 60 m above the surface for L26)
            "u_low" | "v_low" | "wind_low" => {
                let tr = &self.tr;
                let k = self.nlev() - 1;
                let (u, v) = tr.uv_from_vort_div(&self.curr.vor[k], &self.curr.div[k]);
                let nlon = tr.nlon();
                let mut out = Vec::with_capacity(tr.ngrid());
                for j in 0..tr.nlat() {
                    let c = (1.0 / (1.0 - tr.grid.mu[j] * tr.grid.mu[j]).sqrt()) as Real;
                    for i in 0..nlon {
                        let (uu, vv) = (u[j * nlon + i] * c, v[j * nlon + i] * c);
                        out.push(match name {
                            "u_low" => uu,
                            "v_low" => vv,
                            _ => (uu * uu + vv * vv).sqrt(),
                        });
                    }
                }
                one(out)
            }
            // air temperature at the lowest model level [K]
            "t_low" => one(self
                .tr
                .synthesis(&self.curr.tmp[self.nlev() - 1])
                .iter()
                .map(|t| t + self.planet.t_ref as Real)
                .collect()),
            "land" => self
                .surface
                .as_ref()
                .and_then(|s| one(s.land_fraction.clone())),
            "bucket" => self.surface.as_ref().and_then(|s| one(s.bucket.clone())),
            // filtered surface height Φ_s / g [m]
            "orog" => one(self
                .phis_grid
                .iter()
                .map(|p| p / self.planet.gravity as Real)
                .collect()),
            "ts" => self.surface.as_ref().and_then(|s| one(s.ts.clone())),
            // net energy flux into the surface [W m⁻²] (last physics step)
            "surface_heat" if !self.physics.exchange.is_empty() => {
                one(self.exchange_field(crate::physics::driver::Exchange::SFC_NET))
            }
            "psl" => one(self.sea_level_pressure()),
            // precipitable water [kg m⁻² = mm]
            "pw" => self.column_water().and_then(one),
            // Pressure-level fields: <name><hPa>, e.g. t850, u850, v250, phi500.
            _ if PRESSURE_FIELDS.contains(&name) => {
                let split = name.find(|c: char| c.is_ascii_digit())?;
                let (base, hpa) = name.split_at(split);
                let hpa: f64 = hpa.parse().ok()?;
                let f3 = self.level_fields(base)?;
                one(self.to_pressure(&f3, hpa * 100.0))
            }
            // total precipitation = sum of all precip_* diagnostics
            "precip" if reg.id("precip").is_none() => {
                let mut acc: Option<Vec<Real>> = None;
                for &id in reg.precip_parts() {
                    let g = &self.physics.diag[id.0];
                    match acc.as_mut() {
                        None => acc = Some(g.to_vec()),
                        Some(a) => a.iter_mut().zip(g).for_each(|(x, y)| *x += *y),
                    }
                }
                acc.and_then(one)
            }
            _ => {
                if let Some(id) = reg.id(name) {
                    return one(self.physics.diag[id.0].to_vec());
                }
                self.level_fields(name).map(|data| GridField {
                    nlev: self.nlev(),
                    data,
                })
            }
        }
    }

    pub(crate) fn output_diagnostics(&self) -> Vec<(&'static str, f64)> {
        let [mass, energy, am, umax, ke] = self.conservation();
        let mut v = vec![
            ("mass", mass),
            ("energy", energy),
            ("angular_momentum", am),
            ("kinetic_energy", ke),
            ("max_u", umax),
            ("max_courant", self.last_courant),
        ];
        // global means of the physics diagnostics (e.g. olr, asr, precip, evap) and T_s
        let area = 4.0 * std::f64::consts::PI * self.planet.radius * self.planet.radius;
        for (name, grid) in self
            .physics
            .registry
            .names()
            .iter()
            .zip(self.physics.diag.levels())
        {
            v.push((name, self.tr.integrate(grid) / area));
        }
        if let Some(sf) = &self.surface {
            v.push(("ts", self.tr.integrate(&sf.ts) / area));
        }
        // fixed set of names for the whole run (the CSV header is written at step 0)
        if self.surface.is_some() || !self.physics.is_empty() {
            let sh = if self.physics.exchange.is_empty() {
                0.0
            } else {
                let x = self.exchange_field(crate::physics::driver::Exchange::SFC_NET);
                self.tr.integrate(&x) / area
            };
            v.push(("surface_heat", sh));
        }
        // time integrals of the global fluxes (energy and water budgets) [J], [kg]
        if self.has_budget() {
            let b = &self.budget;
            v.extend([
                ("cum_toa_net", b.toa_net),
                ("cum_sfc_net", b.sfc_net),
                ("cum_precip", b.precip),
                ("cum_evap", b.evap),
                ("cum_diffusion_ke", b.diffusion_ke),
                ("cum_fixer", b.fixer),
            ]);
        }
        // latent energy ∫ L q dm [J]
        if let (Some(cond), Some(w)) = (&self.condensable, self.column_water()) {
            v.push(("latent_energy", cond.latent_heat * self.tr.integrate(&w)));
        }
        v
    }
}
