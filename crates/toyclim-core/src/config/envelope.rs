//! The supported envelope (SPEC.md §9.2 P-5): warnings for settings outside it.

/// Warnings for configurations outside the supported envelope (SPEC.md §9.2 P-5).
pub fn envelope_warnings(cfg: &super::Config) -> Vec<String> {
    let mut w = Vec::new();
    let Ok(p) = cfg.planet.resolve() else {
        return w;
    };
    let earth = crate::planet::Planet::earth();
    let mut check = |what: &str, v: f64, lo: f64, hi: f64| {
        if !(lo..=hi).contains(&v) {
            w.push(format!(
                "{what} = {v:.4} is outside the supported range {lo}–{hi} (SPEC.md §9.2 P-5)"
            ));
        }
    };
    check("radius / R_earth", p.radius / earth.radius, 0.5, 2.0);
    check("gravity [m/s²]", p.gravity, 3.0, 25.0);
    check("surface pressure [bar]", p.p0 / 1e5, 0.1, 10.0);
    if p.rotation_rate != 0.0 {
        check(
            "rotation period [h]",
            p.sidereal_day() / 3600.0,
            6.0,
            f64::INFINITY,
        );
    }
    if let Some(o) = &cfg.orbit {
        check("instellation / S_earth", o.instellation / 1361.0, 0.3, 1.5);
        check("obliquity [deg]", o.obliquity, 0.0, 90.0);
        check("eccentricity", o.eccentricity, 0.0, 0.4);
    }
    if let Some(s) = &cfg.system {
        check("stellar T_eff [K]", s.star.t_eff, 2500.0, 7000.0);
        if let Some(c) = &s.composition {
            let co2 = c.get("CO2").copied().unwrap_or(0.0);
            check("CO2 volume fraction", co2, 0.0, 0.01);
        }
    }
    w
}
