//! Lowering of the typed physics and surface blocks of schema 2 to the internal
//! [`crate::config::PhysicsConfig`] and the schema-1 `[surface]` table.

use super::{copy_params, invalid, put, ConfigV2, SurfaceV2};
use crate::config::ConfigError;
use crate::config::SchemeConfig;
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
    let block = |b: &Option<SchemeConfig>| {
        b.as_ref().map(|b| {
            let mut x = b.params.clone();
            x.insert("scheme".into(), Value::from(b.scheme.as_str()));
            Value::Table(x)
        })
    };
    let mut p = Table::new();
    p.insert(
        "radiation".into(),
        block(&c.radiation).ok_or_else(|| invalid("world physics needs a [radiation] block"))?,
    );
    p.insert(
        "boundary_layer".into(),
        block(&c.boundary_layer)
            .ok_or_else(|| invalid("world physics needs a [boundary_layer] block"))?,
    );
    if let Some(v) = block(&c.convection) {
        p.insert("convection".into(), v);
    }
    if let Some(v) = block(&c.clouds) {
        p.insert("clouds".into(), v);
    }
    if let Some(cd) = &c.condensation {
        put(&mut p, "condensation", Value::Table(cd.clone()));
    }
    t.insert("physics".into(), Value::Table(p));
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
