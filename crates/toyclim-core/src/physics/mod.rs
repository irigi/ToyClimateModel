//! Grid-point (column) physics (SPEC.md §5.4).

pub mod column;
pub mod held_suarez;

pub use column::{ColumnIn, ColumnOut, ColumnPhysics};

use crate::planet::Planet;

/// Build the column physics named in the `[forcing]` block (none → adiabatic).
pub fn build(
    forcing: Option<&toml::Table>,
    _planet: &Planet,
) -> Result<Vec<Box<dyn ColumnPhysics>>, String> {
    let Some(f) = forcing else {
        return Ok(Vec::new());
    };
    let kind = f.get("type").and_then(|v| v.as_str()).unwrap_or("none");
    match kind {
        "none" => Ok(Vec::new()),
        other => Err(format!("unknown forcing type {other:?}")),
    }
}
