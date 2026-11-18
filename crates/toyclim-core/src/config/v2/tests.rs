use super::*;
use crate::config::Config;

const DAY: f64 = 86400.0;

/// A minimal schema-2 world (blocks appended by each test).
fn base(extra: &str) -> String {
    format!(
        r#"
schema = 2
model = "primitive"
truncation = 21
levels = "L26"
length = "1day"
output_every = "1day"
{extra}
"#
    )
}

const EARTH: &str = r#"
[star]
t_eff = 5772.0
luminosity = 1.0
mass = 1.0
[orbit]
semi_major_axis = 1.0
eccentricity = 0.0167
obliquity = 23.44
perihelion_longitude = 282.9
start_solar_longitude = 280.0
spin = "free"
[planet]
radius = 6.37122e6
gravity = 9.80616
rotation_period = "23.9345h"
[atmosphere]
surface_pressure = 1.0e5
composition = { N2 = 0.7808, O2 = 0.2095, Ar = 0.0093, CO2 = 400e-6 }
"#;

#[test]
fn earth_example_derives_instellation_year_and_air() {
    let c = Config::parse(&base(EARTH)).unwrap();
    let o = c.orbit.as_ref().unwrap();
    assert!((o.instellation / 1361.0 - 1.0).abs() < 1e-4);
    assert!((o.year.0 / (365.256 * DAY) - 1.0).abs() < 1e-4);
    assert!(!o.tidally_locked);
    let p = c.planet.resolve().unwrap();
    assert!((p.rotation_rate - TAU / (23.9345 * 3600.0)).abs() < 1e-15);
    assert!((p.gas_constant - 287.04).abs() < 0.1, "{}", p.gas_constant);
    assert_eq!(p.p0, 1.0e5);
    // L26 named set and the default T21 time step
    assert_eq!(c.sigma_half.as_deref(), Some(&L26_HALF[..]));
    assert_eq!(c.dt.0, 1800.0);
    let s = c.system.as_ref().unwrap();
    assert_eq!(s.star, Star::sun());
    assert!(
        c.envelope_warnings().is_empty(),
        "{:?}",
        c.envelope_warnings()
    );
}

/// TRAPPIST-1e-like input (SPEC.md P0): a 6.1-day year and about 900 W m⁻² within 5%;
/// synchronous rotation fixes the substellar point.
#[test]
fn trappist1e_synchronous() {
    let c = Config::parse(&base(
        r#"
[star]
t_eff = 2566.0
luminosity = 5.5e-4
mass = 0.09
[orbit]
semi_major_axis = 0.0293
spin = "synchronous"
substellar_lon = 180.0
"#,
    ))
    .unwrap();
    let o = c.orbit.as_ref().unwrap();
    assert!((o.year.0 / DAY - 6.1).abs() < 0.05, "{}", o.year.0 / DAY);
    assert!(
        (o.instellation / 900.0 - 1.0).abs() < 0.05,
        "{}",
        o.instellation
    );
    assert!(o.tidally_locked);
    let p = c.planet.resolve().unwrap();
    assert!((p.rotation_rate - TAU / o.year.0).abs() < 1e-20);
    assert!(
        c.envelope_warnings().is_empty(),
        "{:?}",
        c.envelope_warnings()
    );
}

#[test]
fn eccentric_synchronous_librates_and_resonance() {
    let sync = Config::parse(&base(
        "[orbit]\nsemi_major_axis = 0.1\neccentricity = 0.3\nspin = \"synchronous\"\n",
    ))
    .unwrap();
    assert!(!sync.orbit.as_ref().unwrap().tidally_locked);
    let res = Config::parse(&base(
        "[orbit]\nsemi_major_axis = 0.1\neccentricity = 0.3\nspin = \"resonant\"\nresonance = [3, 2]\n",
    ))
    .unwrap();
    let year = res.orbit.as_ref().unwrap().year.0;
    let p = res.planet.resolve().unwrap();
    assert!((p.rotation_rate - 1.5 * TAU / year).abs() < 1e-20);
    assert!(Config::parse(&base(
        "[orbit]\nsemi_major_axis = 1.0\nspin = \"resonant\"\n"
    ))
    .is_err());
}

#[test]
fn inconsistent_overrides_are_rejected() {
    let with = |extra: &str| base(&format!("[orbit]\nsemi_major_axis = 1.0\n{extra}"));
    // within 1%: accepted and used
    let c = Config::parse(&with("year = \"365.25day\"\ninstellation = 1361.0\n")).unwrap();
    assert_eq!(c.orbit.as_ref().unwrap().year.0, 365.25 * DAY);
    // 2% off: rejected unless allowed
    let e = Config::parse(&with("instellation = 1390.0\n")).unwrap_err();
    assert!(e.to_string().contains("instellation"), "{e}");
    assert!(Config::parse(&with("year = \"370day\"\n")).is_err());
    let ok = Config::parse(&with("instellation = 1390.0\nallow_inconsistent = true\n")).unwrap();
    assert_eq!(ok.orbit.as_ref().unwrap().instellation, 1390.0);
    // rotation of a locked planet checked against the orbit
    let lock = |r: &str| {
        base(&format!(
            "[orbit]\nsemi_major_axis = 0.0293\nspin = \"synchronous\"\n[star]\nt_eff = 2566.0\nluminosity = 5.5e-4\nmass = 0.09\n[planet]\nrotation_rate = {r}\n"
        ))
    };
    assert!(Config::parse(&lock("1.1922e-5")).is_ok());
    assert!(Config::parse(&lock("1.3e-5")).is_err());
    // atmosphere overrides
    let atm = |r: &str| {
        base(&format!(
            "[atmosphere]\nsurface_pressure = 1e5\ncomposition = {{ N2 = 0.78, O2 = 0.21, Ar = 0.01 }}\ngas_constant = {r}\n"
        ))
    };
    assert!(Config::parse(&atm("287.04")).is_ok());
    assert!(Config::parse(&atm("300.0")).is_err());
}

#[test]
fn typed_physics_blocks_lower_to_the_world_physics() {
    let c = Config::parse(&base(
        r#"
initial_condition = { case = "jw06" }
[moisture]
condensable = "water"
[radiation]
scheme = "gray"
albedo = 0.25
[boundary_layer]
scheme = "kprofile"
ri_crit = 1.0
[convection]
scheme = "sbm"
tau = "2h"
[surface]
geography = "aquaplanet"
[surface.ocean]
model = "slab"
mixed_layer_depth = 50.0
[surface.sea_ice]
enabled = false
"#,
    ))
    .unwrap();
    assert!(c.forcing.is_none());
    let p = c.physics.as_ref().unwrap();
    assert_eq!(p.radiation.scheme, "gray");
    assert_eq!(p.radiation.params["albedo"].as_float(), Some(0.25));
    assert_eq!(p.boundary_layer.scheme, "kprofile");
    assert_eq!(
        p.convection.as_ref().unwrap().params["tau"].as_str(),
        Some("2h")
    );
    let s = c.surface.as_ref().unwrap();
    assert_eq!(s.mixed_layer_depth, 50.0);
    assert!(s.ice_albedo < 0.0 && s.heightmap.is_none());
    // the lowered config builds a model
    crate::primitive::from_config(&c).unwrap();
}

#[test]
fn unknown_and_unimplemented_keys_are_errors() {
    for bad in [
        "[radiation]\nscheme = \"gray\"\nbogus = 1\n[boundary_layer]\nscheme = \"kprofile\"\n",
        "[radiation]\nscheme = \"semigray\"\n[boundary_layer]\nscheme = \"kprofile\"\n",
        "[radiation]\nscheme = \"gray\"\n[boundary_layer]\nscheme = \"tj16\"\n",
        "[radiation]\nscheme = \"gray\"\n[boundary_layer]\nscheme = \"kprofile\"\nbogus = 2\n",
        "[forcing]\ntype = \"held_suarez\"\n[radiation]\nscheme = \"gray\"\n",
        "[orbit]\nsemi_major_axis = 1.0\nbogus = 2\n",
        "[surface.ocean]\nmodel = \"wind_driven\"\n",
        "[dynamics.sponge]\nenabled = true\n",
        "[dynamics]\ntracer_transport = \"lagrangian\"\n",
        "[star]\nt_eff = 3000.0\nluminosity = 0.01\nmass = 0.2\n",
    ] {
        assert!(Config::parse(&base(bad)).is_err(), "accepted:\n{bad}");
    }
    let l99 = base("").replace("levels = \"L26\"", "levels = \"L99\"");
    assert!(Config::parse(&l99).unwrap_err().to_string().contains("L99"));
    assert!(Config::parse(&base("").replace("schema = 2", "schema = 3")).is_err());
}

#[test]
fn envelope_warnings_fire_outside_the_envelope() {
    let c = Config::parse(&base(
        "[orbit]\nsemi_major_axis = 0.5\neccentricity = 0.5\n[planet]\nrotation_period = \"-3h\"\n",
    ))
    .unwrap();
    let w = c.envelope_warnings().join("\n");
    assert!(w.contains("instellation"), "{w}");
    assert!(w.contains("eccentricity"), "{w}");
    assert!(w.contains("rotation period"), "{w}");
    // retrograde rotation from a negative period
    assert!(c.planet.resolve().unwrap().rotation_rate < 0.0);
}
