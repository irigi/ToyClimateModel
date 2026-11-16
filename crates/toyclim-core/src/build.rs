//! Construct a model from a run configuration.

use crate::config::{Config, ModelKind};
use crate::model::Model;

/// Build the model named in `cfg`.
/// Settings outside the supported envelope (SPEC.md §9.2 P-5) are logged as warnings.
pub fn build_model(cfg: &Config) -> Result<Box<dyn Model>, String> {
    for w in cfg.envelope_warnings() {
        log::warn!("outside the supported envelope: {w}");
    }
    match cfg.model {
        ModelKind::ShallowWater => Ok(Box::new(crate::shallow_water::from_config(cfg)?)),
        ModelKind::Primitive => Ok(Box::new(crate::primitive::from_config(cfg)?)),
        ModelKind::Dummy => Err("the dummy model has no dynamics".into()),
    }
}
