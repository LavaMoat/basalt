//! Static module record.
//!
//! More information in the [static module record design document](https://github.com/endojs/endo/blob/master/packages/static-module-record/DESIGN.md).
use indexmap::{IndexMap, IndexSet};

use swc_ecma_ast::Module;

use serde::{Serialize, Serializer};

/// Type for live exports.
pub type LiveExport<'a> = (&'a str, bool);

/// Enumeration of the import types.
#[derive(Debug)]
pub enum ImportKind {
    /// A named import specifier.
    Named,
    /// A default import specifier.
    Default,
    /// A wildcard import specifier.
    All,
}

/// Import specifier that may be aliased.
#[derive(Debug)]
pub struct ImportName<'a> {
    name: &'a str,
    alias: Option<&'a str>,
    kind: ImportKind,
}

impl<'a> ImportName<'a> {
    /// Get the raw name for the import respecting the import kind
    /// to return `default` or `*` when necessary.
    pub fn raw_name(&self) -> &str {
        match self.kind {
            ImportKind::Named => &self.name,
            ImportKind::Default => "default",
            ImportKind::All => "*",
        }
    }
}

impl<'a> Serialize for ImportName<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.raw_name())
    }
}

/// Static module record that can be serialized to JSON.
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StaticModuleRecord<'a> {
    /// All exports, eg: `export * from './foo.js';`
    pub export_alls: Vec<&'a str>,
    /// All the imports for the module.
    pub imports: IndexMap<&'a str, Vec<ImportName<'a>>>,
    /// Map of live exports.
    pub live_export_map: IndexMap<&'a str, LiveExport<'a>>,
    /// Map of fixed exports.
    pub fixed_export_map: IndexMap<&'a str, Vec<&'a str>>,

    /// The source module AST node.
    #[serde(skip)]
    pub module: &'a Module,

    /// Exported functions that need some transforms hoisted.
    #[serde(skip)]
    pub hoisted_funcs: IndexSet<String>,

    /// Exported references that need some transforms hoisted.
    #[serde(skip)]
    pub hoisted_refs: IndexSet<String>,
}

/// Owned static module record meta data.
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StaticModuleRecordMeta {
    /// All exports, eg: `export * from './foo.js';`
    pub export_alls: Vec<String>,
    /// All the imports for the module.
    pub imports: IndexMap<String, Vec<String>>,
    /// Map of live exports.
    pub live_export_map: IndexMap<String, (String, bool)>,
    /// Map of fixed exports.
    pub fixed_export_map: IndexMap<String, Vec<String>>,
}

/// Meta data and transformed program combined.
#[derive(Serialize, Debug)]
pub struct StaticModuleRecordProgram {
    /// The meta data.
    pub meta: StaticModuleRecordMeta,
    /// The module transformed to a program functor.
    pub program: String,
}

impl Into<StaticModuleRecordMeta> for StaticModuleRecord<'_> {
    fn into(self) -> StaticModuleRecordMeta {
        StaticModuleRecordMeta {
            export_alls: self
                .export_alls
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>(),
            imports: self
                .imports
                .iter()
                .map(|(k, v)| {
                    (
                        k.to_string(),
                        v.iter()
                            .map(|s| s.raw_name().to_string())
                            .collect::<Vec<_>>(),
                    )
                })
                .collect::<IndexMap<_, _>>(),
            live_export_map: self
                .live_export_map
                .iter()
                .map(|(k, v)| (k.to_string(), (v.0.to_string(), v.1)))
                .collect::<IndexMap<_, _>>(),
            fixed_export_map: self
                .fixed_export_map
                .iter()
                .map(|(k, v)| {
                    (
                        k.to_string(),
                        v.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
                    )
                })
                .collect::<IndexMap<_, _>>(),
        }
    }
}

impl<'a> StaticModuleRecord<'a> {
    /// Get the list of import declarations.
    ///
    /// This is used by the transform to set up the locally
    /// scoped variable names.

    /// FIXME: do not declare re-exports???
    pub fn decls(&self) -> Vec<&str> {
        self.imports
            .iter()
            .map(|(_k, v)| v)
            .flatten()
            .map(|i| {
                if let Some(alias) = i.alias {
                    // Special case when re-exporting as default
                    //
                    // export { meaning as default } from './meaning.js';
                    if alias == "default" {
                        return i.name;
                    }
                    return alias;
                }
                i.name
            })
            .collect::<Vec<_>>()
    }

    /// Get the list of aliases for an import specifier.
    ///
    /// If an alias is not available the name is used instead.
    pub fn aliases(&self) -> IndexMap<&str, Vec<&str>> {
        self.imports
            .iter()
            .map(|(k, v)| {
                (
                    *k,
                    v.iter()
                        .map(|i| i.alias.as_deref().unwrap_or(i.name))
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<IndexMap<_, _>>()
    }
}

pub mod analyzer;
pub mod parser;
pub mod transform;

pub use parser::Parser;
pub use transform::{transform, TransformSource};
