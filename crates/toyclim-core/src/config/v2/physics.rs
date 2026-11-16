//! Lowering of the typed physics and surface blocks of schema 2 to the prototype bundle
//! (`[forcing] type = "gray_aquaplanet"`) and the schema-1 `[surface]` table.

use super::{copy_params, invalid, put, ConfigV2, SurfaceV2};
use crate::config::ConfigError;
use toml::{Table, Value};

pub(super) fn lower_physics(c: &ConfigV2, t: &mut Table) -> Result<(), ConfigError> {
    let typed = c.radiation.is_some()
        || c.boundary_layer.is_some()
        || c.convection.is_some()
        || c.condensation.is_some()
        || c.clouds.is_some();
    if let Some(f) = &c.forcing {
        if typed {
            return Err(invalid(
                "use either [forcing] (an idealized bundle) or the typed physics blocks",
            ));
        }
        t.insert("forcing".into(), Value::Table(f.clone()));
        return Ok(());
    }
    if !typed {
        return Ok(());
    }
    let not_yet = |what: &str| invalid(format!("{what} is not implemented yet"));
    let mut f = Table::new();
    put(&mut f, "type", "gray_aquaplanet");
    let rad = c
        .radiation
        .as_ref()
        .ok_or_else(|| invalid("world physics needs a [radiation] block"))?;
    match rad.scheme.as_str() {
        "gray" => copy_params(
            "radiation",
            &rad.params,
            &[
                ("tau_eq", "tau_eq"),
                ("tau_pole", "tau_pole"),
                ("f_linear", "f_linear"),
                ("tau_q", "tau_q"),
                ("tau_sw", "tau_sw"),
                ("albedo", "albedo"),
                ("t_surface_default", "sst_uniform"),
            ],
            &mut f,
        )?,
        "semigray" => return Err(not_yet("radiation scheme \"semigray\" (P3)")),
        other => return Err(invalid(format!("unknown radiation scheme {other:?}"))),
    }
    let bl = c
        .boundary_layer
        .as_ref()
        .ok_or_else(|| invalid("world physics needs a [boundary_layer] block"))?;
    match bl.scheme.as_str() {
        "tj16" => copy_params(
            "boundary_layer",
            &bl.params,
            &[
                ("c_h", "c_h"),
                ("c_e", "c_e"),
                ("p_pbl", "p_pbl"),
                ("p_strato", "p_strato"),
                ("drag_time", "tau_friction"),
                ("drag_sigma_b", "sigma_b"),
            ],
            &mut f,
        )?,
        "kprofile" => return Err(not_yet("boundary_layer scheme \"kprofile\" (P1)")),
        other => return Err(invalid(format!("unknown boundary_layer scheme {other:?}"))),
    }
    match &c.convection {
        None => put(&mut f, "convection", "none"),
        Some(c) => match c.scheme.as_str() {
            "sbm" | "none" => {
                put(&mut f, "convection", c.scheme.as_str());
                copy_params(
                    "convection",
                    &c.params,
                    &[("tau", "tau_convection"), ("rh_ref", "rh_convection")],
                    &mut f,
                )?
            }
            "dry" => return Err(not_yet("convection scheme \"dry\" (P1)")),
            other => return Err(invalid(format!("unknown convection scheme {other:?}"))),
        },
    }
    if c.condensation.as_ref().is_some_and(|c| c.reevaporation) {
        return Err(not_yet("condensation.reevaporation (P2)"));
    }
    if let Some(c) = &c.clouds {
        match c.scheme.as_str() {
            "none" => {}
            "rh_prototype" => {
                put(&mut f, "clouds", true);
                copy_params(
                    "clouds",
                    &c.params,
                    &[
                        ("rh_crit", "cloud_rh_crit"),
                        ("sw_reflect", "cloud_sw_reflect"),
                        ("lw_tau", "cloud_lw_tau"),
                    ],
                    &mut f,
                )?
            }
            "diagnostic" => return Err(not_yet("clouds scheme \"diagnostic\" (P4)")),
            other => return Err(invalid(format!("unknown clouds scheme {other:?}"))),
        }
    }
    t.insert("forcing".into(), Value::Table(f));
    Ok(())
}

pub(super) fn lower_surface(s: &SurfaceV2) -> Result<Table, ConfigError> {
    let mut t = Table::new();
    if s.geography != "aquaplanet" {
        put(&mut t, "heightmap", s.geography.as_str());
    }
    if let Some(x) = s.orography_filter {
        put(&mut t, "orography_filter", x);
    }
    let empty = Table::new();
    let ocean = s.ocean.as_ref().unwrap_or(&empty);
    let mut ocean_map = vec![
        ("mixed_layer_depth", "mixed_layer_depth"),
        ("heat_transport", "ocean_heat_transport"),
        ("albedo", "ocean_albedo"),
        ("initial", "initial"),
    ];
    match ocean.get("model").and_then(Value::as_str).unwrap_or("slab") {
        "slab" => {}
        "fixed_sst" => put(&mut t, "fixed", true),
        "wind_driven" => {
            return Err(invalid(
                "ocean model \"wind_driven\" is not implemented yet (P6)",
            ))
        }
        other => return Err(invalid(format!("unknown ocean model {other:?}"))),
    }
    ocean_map.push(("model", ""));
    for (k, v) in ocean {
        let (_, to) = ocean_map
            .iter()
            .find(|(from, _)| from == k)
            .ok_or_else(|| invalid(format!("unknown key {k:?} in [surface.ocean]")))?;
        if !to.is_empty() {
            t.insert(to.to_string(), v.clone());
        }
    }
    if let Some(land) = &s.land {
        copy_params(
            "surface.land",
            land,
            &[
                ("heat_capacity", "land_heat_capacity"),
                ("albedo", "land_albedo"),
                ("bucket_capacity", "bucket_capacity"),
                ("initial_bucket", "initial_bucket"),
            ],
            &mut t,
        )?;
    }
    // Prototype sea ice: an albedo switch below a freezing temperature (P2 replaces it).
    if let Some(ice) = &s.sea_ice {
        let mut ice = ice.clone();
        let enabled = ice
            .remove("enabled")
            .map(|v| {
                v.as_bool()
                    .ok_or_else(|| invalid("sea_ice.enabled must be a bool"))
            })
            .transpose()?
            .unwrap_or(true);
        copy_params(
            "surface.sea_ice",
            &ice,
            &[
                ("albedo", "ice_albedo"),
                ("ocean_freeze", "ocean_freeze"),
                ("land_freeze", "land_freeze"),
            ],
            &mut t,
        )?;
        if !enabled {
            put(&mut t, "ice_albedo", -1.0);
        }
    }
    Ok(t)
}
