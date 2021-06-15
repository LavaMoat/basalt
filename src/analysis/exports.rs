//! Helper to analyze exports from a module.
use swc_ecma_ast::*;
use swc_ecma_visit::{Node, Visit};

#[derive(Debug)]
pub enum ExportRecord {
    VarDecl { var: VarDecl },
    FnDecl { func: FnDecl },
    Named { specifiers: Vec<ExportSpecifier> },
    DefaultExpr { expr: Box<Expr> },
}

#[derive(Debug)]
pub enum ReexportRecord {
    All {
        module_path: String,
    },
    Named {
        module_path: String,
        specifiers: Vec<ExportSpecifier>,
    },
}

#[derive(Default, Debug)]
pub struct ExportAnalysis {
    pub exports: Vec<ExportRecord>,
    pub reexports: Vec<ReexportRecord>,
    pub live: Vec<String>,
}

impl ExportAnalysis {
    pub fn new() -> Self {
        Self {
            exports: Default::default(),
            reexports: Default::default(),
            live: Default::default(),
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

impl Visit for ExportAnalysis {
    fn visit_module_item(&mut self, n: &ModuleItem, _: &dyn Node) {
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
                _ => {
                    //println!("unhandled node: {:#?}", decl);
                }
            },
            _ => {}
        }
    }
}
