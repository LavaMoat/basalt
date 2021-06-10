//! Static module record.
//!
//! More information in the [static module record design document](https://github.com/endojs/endo/blob/master/packages/static-module-record/DESIGN.md).
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Type for live exports.
pub type LiveExport = (String, bool);

/// Static module record that can be serialized to JSON.
#[derive(Serialize, Deserialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StaticModuleRecord {
    /// All exports, eg: `export * from './foo.js';`
    pub export_alls: Vec<String>,
    /// All the imports for the module.
    pub imports: HashMap<String, Vec<String>>,
    /// Map of live exports.
    pub live_export_map: HashMap<String, LiveExport>,
    /// Map of fixed exports.
    pub fixed_export_map: HashMap<String, Vec<String>>,
}

pub mod generator;
pub mod parser;

pub use generator::Generator;
pub use parser::Parser;
