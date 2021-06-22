//! Helpers to analyze modules.
use swc_ecma_ast::*;
use swc_ecma_visit::{Node, Visit};

use indexmap::{IndexMap, IndexSet};

/// Record of an import definition.
#[derive(Debug)]
pub enum ImportRecord {
    /// Side effect import, no specifiers.
    None,
    /// Wildcard import.
    All {
        /// A name for the import specifier.
        name: String,
    },
    /// Default import.
    Default {
        /// A name for the import specifier.
        name: String,
    },
    /// Import specifier with optional alias.
    ///
    /// If no alias is given it will match the name.
    Named {
        /// A name for the import specifier.
        name: String,
        /// An alias for the import specifier after the `as` keyword.
        alias: String,
    },
}

/// Record for a module export.
#[derive(Debug)]
pub enum ExportRecord {
    /// Variable export.
    VarDecl {
        /// The exported variable declaration.
        var: VarDecl,
    },
    /// Function export.
    FnDecl {
        /// The exported function declaration.
        func: FnDecl,
    },
    /// Named export.
    Named {
        /// The exported specifiers.
        specifiers: Vec<ExportSpecifier>,
    },
    /// Default export.
    DefaultExpr {
        /// The exported expression.
        expr: Box<Expr>,
    },
}

/// Record for a module re-export.
#[derive(Debug)]
pub enum ReexportRecord {
    /// Re-export all symbols with a wildcard.
    All {
        /// The module path.
        module_path: String,
    },
    /// Re-export named specifiers.
    Named {
        /// The module path.
        module_path: String,
        /// List of export specifiers.
        specifiers: Vec<ExportSpecifier>,
    },
}

/// Analyze a module's exports.
#[derive(Default, Debug)]
pub struct Analyzer {
    /// List of computed imports.
    pub imports: IndexMap<String, Vec<ImportRecord>>,
    /// List of computed exports.
    pub exports: Vec<ExportRecord>,
    /// List of computed re-exports.
    pub reexports: Vec<ReexportRecord>,
    /// Functions that need some transforms hoisted.
    pub hoisted_funcs: IndexSet<String>,
}

impl Analyzer {
    /// Create a new export analyzer.
    pub fn new() -> Self {
        Self {
            imports: Default::default(),
            exports: Default::default(),
            reexports: Default::default(),
            hoisted_funcs: Default::default(),
        }
    }

    /// Get the names of exported symbols so that the live export
    /// analysis can detect which exports have assignment.
    pub fn var_export_names(&self) -> Vec<String> {
        let mut out = Vec::new();
        for rec in self.exports.iter() {
            match rec {
                ExportRecord::VarDecl { var } => {
                    for decl in var.decls.iter() {
                        match &decl.name {
                            Pat::Ident(ident) => {
                                out.push(ident.id.sym.as_ref().to_string());
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        out
    }
}

impl Visit for Analyzer {
    fn visit_module_item(&mut self, n: &ModuleItem, node: &dyn Node) {
        match n {
            ModuleItem::ModuleDecl(decl) => match decl {
                // export * from 'import-and-export-all.js';
                ModuleDecl::ExportAll(export) => {
                    let module_path = export.src.value.as_ref().to_string();
                    self.reexports.push(ReexportRecord::All { module_path });
                }
                ModuleDecl::ExportNamed(export) => {
                    // export { grey as gray } from './reexport-name-and-rename.js';
                    if let Some(ref src) = export.src {
                        let module_path = src.value.as_ref().to_string();
                        let specifiers = export.specifiers.clone();
                        self.reexports.push(ReexportRecord::Named {
                            module_path,
                            specifiers,
                        });
                    // export { aleph as alpha };
                    } else {
                        let specifiers = export.specifiers.clone();
                        self.exports.push(ExportRecord::Named { specifiers });
                    }
                }
                ModuleDecl::ExportDecl(export) => match &export.decl {
                    // export const foo = null;
                    Decl::Var(var) => {
                        self.exports
                            .push(ExportRecord::VarDecl { var: var.clone() });
                    }
                    // export function foo() {}
                    Decl::Fn(func) => {
                        self.hoisted_funcs
                            .insert(func.ident.sym.as_ref().to_string());
                        self.exports
                            .push(ExportRecord::FnDecl { func: func.clone() });
                    }
                    _ => {}
                },
                ModuleDecl::ExportDefaultExpr(export) => {
                    self.exports.push(ExportRecord::DefaultExpr {
                        expr: export.expr.clone(),
                    });
                }
                ModuleDecl::Import(import) => {
                    self.visit_import_decl(import, node);
                }
                _ => {
                    //println!("unhandled node: {:#?}", decl);
                }
            },
            _ => {}
        }
    }

    fn visit_import_decl(&mut self, n: &ImportDecl, _: &dyn Node) {
        let module_path = n.src.value.as_ref().to_string();

        // No specifiers is a side effect import, eg: `import "module";`
        if n.specifiers.is_empty() {
            let list = self
                .imports
                .entry(module_path.clone())
                .or_insert(Vec::new());
            list.push(ImportRecord::None);
        } else {
            for spec in n.specifiers.iter() {
                let list = self
                    .imports
                    .entry(module_path.clone())
                    .or_insert(Vec::new());
                match spec {
                    ImportSpecifier::Namespace(item) => {
                        list.push(ImportRecord::All {
                            name: item.local.sym.as_ref().to_string(),
                        });
                    }
                    ImportSpecifier::Default(item) => {
                        list.push(ImportRecord::Default {
                            name: item.local.sym.as_ref().to_string(),
                        });
                    }
                    ImportSpecifier::Named(item) => {
                        let alias = item.local.sym.as_ref().to_string();
                        let name = item
                            .imported
                            .as_ref()
                            .map(|n| n.sym.as_ref().to_string())
                            .unwrap_or(item.local.sym.as_ref().to_string());
                        list.push(ImportRecord::Named { name, alias });
                    }
                }
            }
        }
    }
}

/// Live export analysis is done as a separate pass from the
/// export analysis which will be slower but makes the code a lot
/// easier to reason about.
///
/// The export analysis needs to use `visit_module_item()` to
/// detect the exports correctly but this means we would need
/// to branch in many places to detect all the variants for where
/// statements could appear so we detect the statements in a separate
/// visitor pass.
#[derive(Default, Debug)]
pub struct LiveExportAnalysis {
    /// List of exported symbol names.
    pub exports: Vec<String>,
    /// List of exported symbol names that are considered live exports.
    pub live: Vec<String>,
    /// List of export references that should be hoisted during transformation.
    pub hoisted_refs: IndexSet<String>,
}

impl LiveExportAnalysis {
    /// Create a new live export analyzer.
    pub fn new() -> Self {
        Self {
            exports: Default::default(),
            live: Default::default(),
            hoisted_refs: Default::default(),
        }
    }
}

impl LiveExportAnalysis {
    fn detect_match(&mut self, sym: &str) -> Option<String> {
        let matched = self
            .exports
            .iter()
            .find(|name| sym == *name);
        if matched.is_some() {
            self.live.push(sym.to_string());
        }
        matched.map(|m| m.to_string())
    }
}

impl Visit for LiveExportAnalysis {

    fn visit_expr(&mut self, n: &Expr, _: &dyn Node) {
        match n {
            // ++i, i++, --i, i--
            Expr::Update(expr) => match &*expr.arg {
                Expr::Ident(ident) => {
                    self.detect_match(ident.sym.as_ref());
                }
                _ => {}
            },
            // count = 1
            Expr::Assign(expr) => {
                match &expr.left {
                    PatOrExpr::Pat(pat) => match &**pat {
                        Pat::Ident(ident) => {
                            self.detect_match(ident.id.sym.as_ref());
                        }
                        _ => {}
                    },
                    PatOrExpr::Expr(expr) => match &**expr {
                        Expr::Ident(ident) => {
                            self.detect_match(ident.sym.as_ref());
                        }
                        _ => {}
                    },
                }
            }
            // export const abc2 = abc;
            Expr::Ident(ident) => {
                let mut matched = self.detect_match(ident.sym.as_ref());
                if let Some(matched) = matched.take() {
                    self.hoisted_refs.insert(matched);
                }
            }
            _ => {},
        }
    }

}
