//! Static module record.
//!
//! More information in the [static module record design document](https://github.com/endojs/endo/blob/master/packages/static-module-record/DESIGN.md).
use std::collections::HashMap;

use swc_ecma_ast::Module;

use serde::{Deserialize, Serialize};

/// Type for live exports.
pub type LiveExport<'a> = (&'a str, bool);

/// Static module record that can be serialized to JSON.
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StaticModuleRecord<'a> {
    /// All exports, eg: `export * from './foo.js';`
    pub export_alls: Vec<&'a str>,
    /// All the imports for the module.
    pub imports: HashMap<&'a str, Vec<&'a str>>,
    /// Map of live exports.
    pub live_export_map: HashMap<&'a str, LiveExport<'a>>,
    /// Map of fixed exports.
    pub fixed_export_map: HashMap<&'a str, Vec<&'a str>>,

    /// The source module AST node.
    #[serde(skip)]
    pub module: &'a Module,

    /// List of import declarations.
    #[serde(skip)]
    pub import_decls: Vec<&'a str>,

    /// Map from import to declaration names (specifiers).
    #[serde(skip)]
    pub import_alias: HashMap<&'a str, Vec<&'a str>>,
}

pub mod generator;
pub mod parser;
pub mod transform;

pub use generator::Generator;
pub use parser::Parser;
pub use transform::transform;
