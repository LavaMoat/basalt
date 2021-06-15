//! Static module record.
//!
//! More information in the [static module record design document](https://github.com/endojs/endo/blob/master/packages/static-module-record/DESIGN.md).
use std::collections::HashMap;

use swc_ecma_ast::Module;

use serde::{Serialize, Serializer};

/// Type for live exports.
pub type LiveExport<'a> = (&'a str, bool);

/// Import specifier that may be aliased.
#[derive(Debug)]
pub struct ImportName<'a> {
    name: &'a str,
    alias: Option<&'a str>,
}

impl<'a> Serialize for ImportName<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.name)
    }
}

/// Static module record that can be serialized to JSON.
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StaticModuleRecord<'a> {
    /// All exports, eg: `export * from './foo.js';`
    pub export_alls: Vec<&'a str>,
    /// All the imports for the module.
    pub imports: HashMap<&'a str, Vec<ImportName<'a>>>,
    /// Map of live exports.
    pub live_export_map: HashMap<&'a str, LiveExport<'a>>,
    /// Map of fixed exports.
    pub fixed_export_map: HashMap<&'a str, Vec<&'a str>>,

    /// The source module AST node.
    #[serde(skip)]
    pub module: &'a Module,

    /// List of import declarations.
    ///
    /// This is used by the transform to set up the locally
    /// scoped variable names.
    #[serde(skip)]
    pub import_decls: Vec<&'a str>,

    /// Map from import to declaration names (specifiers).
    #[serde(skip)]
    pub import_alias: HashMap<&'a str, Vec<&'a str>>,
}

impl<'a> StaticModuleRecord<'a> {

    /// Get the list of import declarations.
    ///
    /// This is used by the transform to set up the locally
    /// scoped variable names.
    pub fn decls(&self) -> Vec<&'a str> {
        self.imports
            .iter()
            .map(|(k, v)| v )
            .flatten()
            .map(|i| i.name)
            .collect::<Vec<_>>()
    }
}

pub mod generator;
pub mod parser;
pub mod transform;

pub use generator::Generator;
pub use parser::Parser;
pub use transform::transform;
