//! The column physics contract (SPEC.md §9.3.3): a read-only context, the provisional
//! column state that each scheme updates in place, what the atmosphere and the surface
//! exchange, and per-worker scratch space.
//!
//! **Sequential (time-split) update.** The driver copies the column into a [`ColumnState`].
//! Each scheme reads the state as left by the schemes before it and changes it over the
//! physics step Δt_p through [`ColumnState::add_t`] / [`ColumnState::set_t`] (and the u, v,
//! q variants). These keep the provisional value and the accumulated tendency
//! (final − initial)/Δt_p together; the accumulated tendency is what the dynamics gets.
//!
//! Index 0 is the top level throughout. Column quantities are `f64` in every build.

use super::condensation::Condensable;
use crate::planet::Planet;
use crate::surface::Tile;

/// Read-only context of one column.
pub struct ColumnCtx<'a> {
    /// Latitude, longitude \[rad\].
    pub lat: f64,
    pub lon: f64,
    /// Column index (row-major Gaussian grid).
    pub index: usize,
    /// Full and half σ levels (L and L+1 values; half level 0 is the top, σ = 0).
    pub sigma: &'a [f64],
    pub sigma_half: &'a [f64],
    /// Surface pressure \[Pa\].
    pub ps: f64,
    /// Full-level pressure p_k, layer thickness Δp_k and half-level pressure \[Pa\].
    pub p: &'a [f64],
    pub dp: &'a [f64],
    pub p_half: &'a [f64],
    /// Surface geopotential Φ_s \[m² s⁻²\].
    pub phis: f64,
    /// Model time \[s\] and the physics step Δt_p \[s\].
    pub time: f64,
    pub dt: f64,
    pub planet: &'a Planet,
    /// The condensable of tracer 0, if the model is moist.
    pub cond: Option<&'a Condensable>,
    /// Stellar flux on a horizontal surface at the top \[W m⁻²\] and cos zenith (0 at night).
    pub insolation: f64,
    pub cos_zenith: f64,
    /// Vertical pressure velocity ω \[Pa s⁻¹\] at the full levels, from the state the physics
    /// sees (σ-coordinate continuity, the dynamics' ω/p times p_k). Filled only when a scheme
    /// asks for it ([`ColumnScheme::needs_omega`]); empty otherwise (and in the SCM).
    pub omega: &'a [f64],
}

/// The provisional column state and the tendencies accumulated so far.
#[derive(Debug, Clone)]
pub struct ColumnState {
    pub nlev: usize,
    /// Zonal and meridional wind \[m s⁻¹\], temperature \[K\].
    pub u: Vec<f64>,
    pub v: Vec<f64>,
    pub t: Vec<f64>,
    /// Tracers `[tracer][lev]` flattened (tracer 0 is the condensable's specific humidity).
    pub q: Vec<f64>,
    /// Accumulated tendencies (final − initial)/Δt_p.
    pub du: Vec<f64>,
    pub dv: Vec<f64>,
    pub dt: Vec<f64>,
    pub dq: Vec<f64>,
    /// Condensate produced per layer during this step and not yet fallen out, liquid and
    /// ice \[kg m⁻² s⁻¹\] (convection and condensation add, the fall scheme removes).
    pub prod_rain: Vec<f64>,
    pub prod_snow: Vec<f64>,
    /// Convection of this step (written by the convection scheme, read by the cloud
    /// diagnosis): precipitation \[kg m⁻² s⁻¹\] and the highest convecting level (index from
    /// the top; `nlev` = no convection). The convecting layer reaches the lowest level.
    pub conv_precip: f64,
    pub conv_top: usize,
    dtp: f64,
}

impl ColumnState {
    pub fn new(nlev: usize, ntracers: usize) -> Self {
        let z = vec![0.0; nlev];
        let zq = vec![0.0; nlev * ntracers];
        ColumnState {
            nlev,
            u: z.clone(),
            v: z.clone(),
            t: z.clone(),
            q: zq.clone(),
            du: z.clone(),
            dv: z.clone(),
            prod_rain: z.clone(),
            prod_snow: z.clone(),
            dt: z,
            dq: zq,
            conv_precip: 0.0,
            conv_top: nlev,
            dtp: 1.0,
        }
    }
    /// Start a new physics step of length `dtp` (zero the accumulated tendencies).
    pub fn begin(&mut self, dtp: f64) {
        self.dtp = dtp;
        for x in [
            &mut self.du,
            &mut self.dv,
            &mut self.dt,
            &mut self.dq,
            &mut self.prod_rain,
            &mut self.prod_snow,
        ] {
            x.fill(0.0);
        }
        self.conv_precip = 0.0;
        self.conv_top = self.nlev;
    }
    /// Record condensate `m` \[kg m⁻² s⁻¹\] formed in layer k with ice fraction `ice`.
    pub fn produce(&mut self, k: usize, m: f64, ice: f64) {
        self.prod_snow[k] += ice * m;
        self.prod_rain[k] += (1.0 - ice) * m;
    }
    /// The physics step Δt_p \[s\].
    pub fn dtp(&self) -> f64 {
        self.dtp
    }
    pub fn ntracers(&self) -> usize {
        self.q.len().checked_div(self.nlev).unwrap_or(0)
    }
    /// Tracer `it` (slice of L values).
    pub fn tracer(&self, it: usize) -> &[f64] {
        &self.q[it * self.nlev..(it + 1) * self.nlev]
    }
    /// Specific humidity of the condensable (tracer 0), if any.
    pub fn humidity(&self) -> Option<&[f64]> {
        (self.q.len() >= self.nlev && self.nlev > 0).then(|| &self.q[..self.nlev])
    }

    /// Add a temperature tendency `rate` \[K s⁻¹\] at level k (applied over Δt_p).
    pub fn add_t(&mut self, k: usize, rate: f64) {
        self.dt[k] += rate;
        self.t[k] += rate * self.dtp;
    }
    pub fn add_u(&mut self, k: usize, rate: f64) {
        self.du[k] += rate;
        self.u[k] += rate * self.dtp;
    }
    pub fn add_v(&mut self, k: usize, rate: f64) {
        self.dv[k] += rate;
        self.v[k] += rate * self.dtp;
    }
    /// Add a tendency of tracer `it` at level k.
    pub fn add_q(&mut self, it: usize, k: usize, rate: f64) {
        let i = it * self.nlev + k;
        self.dq[i] += rate;
        self.q[i] += rate * self.dtp;
    }
    /// Set the temperature at level k to `new` (the change counts as a tendency).
    pub fn set_t(&mut self, k: usize, new: f64) {
        self.dt[k] += (new - self.t[k]) / self.dtp;
        self.t[k] = new;
    }
    pub fn set_u(&mut self, k: usize, new: f64) {
        self.du[k] += (new - self.u[k]) / self.dtp;
        self.u[k] = new;
    }
    pub fn set_v(&mut self, k: usize, new: f64) {
        self.dv[k] += (new - self.v[k]) / self.dtp;
        self.v[k] = new;
    }
    pub fn set_q(&mut self, it: usize, k: usize, new: f64) {
        let i = it * self.nlev + k;
        self.dq[i] += (new - self.q[i]) / self.dtp;
        self.q[i] = new;
    }
}

/// What the atmosphere and the surface of one column trade during a physics step
/// (SPEC.md §9.3.3). Fluxes in W m⁻² and kg m⁻² s⁻¹; signs as in §9.3.4 (turbulent fluxes
/// positive upward, radiative fluxes positive downward).
#[derive(Debug, Clone, Default)]
pub struct SurfaceExchange {
    /// Surface tiles of this cell (empty when the model has no surface; then schemes
    /// that need one use their own prescription).
    pub tiles: Vec<Tile>,
    /// Friction velocity of the previous step \[m s⁻¹\] (ocean roughness, Charnock).
    pub ustar_prev: f64,
    // radiation → surface
    /// Downward shortwave and longwave flux at the surface.
    pub sw_down: f64,
    pub lw_down: f64,
    /// Net downward flux at the top of the atmosphere (absorbed stellar − outgoing).
    pub toa_net: f64,
    // condensation → surface
    /// Rain and snow reaching the surface.
    pub rain: f64,
    pub snow: f64,
    // surface → atmosphere (area means over the tiles)
    /// Sensible heat flux, evaporation, latent heat flux.
    pub shf: f64,
    pub evap: f64,
    pub lhf: f64,
    /// Surface stress on the atmosphere's momentum (momentum flux into the surface).
    pub taux: f64,
    pub tauy: f64,
    /// Friction velocity of this step \[m s⁻¹\].
    pub ustar: f64,
    /// Change of the surface's upward longwave emission by the implicit surface solve
    /// (4σT_s³ΔT_s, area mean). It leaves through the top of the atmosphere, so the driver
    /// adds it to `olr` and subtracts it from `toa_net`.
    pub lw_up_change: f64,
    /// Net energy flux into the surface (radiation − turbulent fluxes).
    pub sfc_net: f64,
    /// Clouds of this column (SPEC.md §9.3.3 step 6 → step 1 of the next physics call),
    /// kept by the driver between steps: `[frac (L) | liquid path (L) | ice path (L)]`,
    /// fraction per layer and in-cloud water paths \[kg m⁻²\]. Empty without a cloud scheme.
    pub cloud: Vec<f64>,
}

impl SurfaceExchange {
    /// Reset the per-step fluxes (tiles are refilled by the driver).
    /// The driver's per-column memory (`cloud`) is kept.
    pub fn begin(&mut self) {
        let tiles = std::mem::take(&mut self.tiles);
        let cloud = std::mem::take(&mut self.cloud);
        *self = SurfaceExchange {
            tiles,
            cloud,
            ..Default::default()
        };
    }
    /// Tile-area-weighted surface albedo (broadband).
    pub fn albedo(&self) -> f64 {
        self.tiles.iter().map(|t| t.frac * t.albedo).sum()
    }
    /// Upward longwave emission σ⟨T_s⁴⟩ of the tiles \[W m⁻²\].
    pub fn lw_emission(&self) -> f64 {
        use super::radiation::STEFAN_BOLTZMANN;
        self.tiles
            .iter()
            .map(|t| t.frac * STEFAN_BOLTZMANN * t.ts.powi(4))
            .sum()
    }
    /// Tile-mean surface temperature \[K\].
    pub fn ts(&self) -> f64 {
        self.tiles.iter().map(|t| t.frac * t.ts).sum()
    }
}

/// Scratch space of one worker, allocated once at model build (no heap allocation per
/// column). Schemes use the buffers they need; none keeps data between calls.
#[derive(Debug, Clone)]
pub struct Workspace {
    /// Buffers of length 2(L + 1) at least.
    pub buf: [Vec<f64>; 12],
}

impl Workspace {
    pub fn new(nlev: usize) -> Self {
        let n = 2 * (nlev + 1);
        Workspace {
            buf: std::array::from_fn(|_| vec![0.0; n]),
        }
    }
}

/// A column physics scheme.
pub trait ColumnScheme: Send + Sync {
    /// Short name (logs, conservation tests).
    fn name(&self) -> &'static str;
    /// Names of the per-column diagnostics this scheme writes, in slot order.
    fn diagnostics(&self) -> &'static [&'static str] {
        &[]
    }
    /// Length of the per-column memory this scheme keeps between physics steps in
    /// [`SurfaceExchange::cloud`] (the driver keeps one buffer of the largest length, saved
    /// in restarts).
    fn memory(&self, _nlev: usize) -> usize {
        0
    }
    /// Whether this scheme reads [`ColumnCtx::omega`] (the driver then computes it).
    fn needs_omega(&self) -> bool {
        false
    }
    /// Update `state` over Δt_p = `ctx.dt`, read and write the surface exchange, and write
    /// this scheme's diagnostics to `diag` (zeroed by the driver).
    fn apply(
        &self,
        ctx: &ColumnCtx,
        state: &mut ColumnState,
        sfc: &mut SurfaceExchange,
        diag: &mut [f64],
        work: &mut Workspace,
    );
}

/// Height of the lowest full level above the surface, R T_v/g · ln(p_s / p_L) \[m\].
pub fn lowest_level_height(ctx: &ColumnCtx, tv: f64) -> f64 {
    let l = ctx.p.len();
    ctx.planet.gas_constant * tv / ctx.planet.gravity * (ctx.ps / ctx.p[l - 1]).ln()
}

/// Test helpers: a column context with its pressure arrays.
#[cfg(test)]
pub mod testing {
    use super::*;

    /// Pressure arrays of a column on σ half levels `half` at surface pressure `ps`.
    pub struct Geometry {
        pub sigma: Vec<f64>,
        pub half: Vec<f64>,
        pub p: Vec<f64>,
        pub dp: Vec<f64>,
        pub ph: Vec<f64>,
        pub ps: f64,
    }

    impl Geometry {
        pub fn new(half: &[f64], ps: f64) -> Self {
            let lev = crate::primitive::vertical::SigmaLevels::from_half(half.to_vec());
            let p = lev.full.iter().map(|s| s * ps).collect();
            let dp = lev.dsigma.iter().map(|d| d * ps).collect();
            let ph = half.iter().map(|s| s * ps).collect();
            Geometry {
                sigma: lev.full,
                half: half.to_vec(),
                p,
                dp,
                ph,
                ps,
            }
        }
        /// L26 levels at 1000 hPa.
        pub fn l26() -> Self {
            Self::new(&crate::primitive::vertical::L26_HALF, 1.0e5)
        }
        pub fn ctx<'a>(
            &'a self,
            planet: &'a Planet,
            cond: Option<&'a Condensable>,
            dt: f64,
        ) -> ColumnCtx<'a> {
            ColumnCtx {
                lat: 0.0,
                lon: 0.0,
                index: 0,
                sigma: &self.sigma,
                sigma_half: &self.half,
                ps: self.ps,
                p: &self.p,
                dp: &self.dp,
                p_half: &self.ph,
                phis: 0.0,
                time: 0.0,
                dt,
                planet,
                cond,
                insolation: 0.0,
                cos_zenith: 0.0,
                omega: &[],
            }
        }
    }
}
