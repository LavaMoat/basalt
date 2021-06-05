//! Helper to analyize imports and exports from a module
use std::collections::HashMap;
use swc_ecma_ast::*;
use swc_ecma_visit::{Node, Visit};

#[derive(Debug)]
pub enum ImportRecord {
    All {
        local: String,
    },
    Default {
        local: String,
    },
    Named {
        local: String,
        alias: Option<String>,
    },
}

impl ImportRecord {
    pub fn word(&self) -> String {
        match self {
            ImportRecord::All { .. } => String::from("*"),
            ImportRecord::Default { .. } => String::from("default"),
            ImportRecord::Named { local, alias } => {
                alias.clone().unwrap_or(local.clone())
            }
        }
    }
}

#[derive(Debug)]
pub enum ExportRecord {
    VarDecl { var: VarDecl },
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
pub struct ImportAnalysis {
    pub imports: HashMap<String, Vec<ImportRecord>>,
}

impl ImportAnalysis {
    pub fn new() -> Self {
        Self {
            imports: Default::default(),
        }
    }
}

impl Visit for ImportAnalysis {
    fn visit_import_decl(&mut self, n: &ImportDecl, _: &dyn Node) {
        let module_path = format!("{}", n.src.value);
        for spec in n.specifiers.iter() {
            let list = self
                .imports
                .entry(module_path.clone())
                .or_insert(Vec::new());
            match spec {
                ImportSpecifier::Namespace(item) => {
                    list.push(ImportRecord::All {
                        local: format!("{}", item.local.sym),
                    });
                }
                ImportSpecifier::Default(item) => {
                    list.push(ImportRecord::Default {
                        local: format!("{}", item.local.sym),
                    });
                }
                ImportSpecifier::Named(item) => {
                    list.push(ImportRecord::Named {
                        local: format!("{}", item.local.sym),
                        alias: item
                            .imported
                            .as_ref()
                            .map(|ident| format!("{}", ident.sym)),
                    });
                }
            }
        }
    }
}

#[derive(Default, Debug)]
pub struct ExportAnalysis {
    pub exports: Vec<ExportRecord>,
    pub reexports: Vec<ReexportRecord>,
}

impl ExportAnalysis {
    pub fn new() -> Self {
        Self {
            exports: Default::default(),
            reexports: Default::default(),
        }
    }
}

impl Visit for ExportAnalysis {
    fn visit_module_item(&mut self, n: &ModuleItem, _: &dyn Node) {
        //println!("{:#?}", n);
        match n {
            ModuleItem::ModuleDecl(decl) => match decl {
                // export * from 'import-and-export-all.js';
                ModuleDecl::ExportAll(export) => {
                    let module_path = format!("{}", export.src.value);
                    self.reexports.push(ReexportRecord::All { module_path });
                }
                ModuleDecl::ExportNamed(export) => {
                    // export { grey as gray } from './reexport-name-and-rename.js';
                    if let Some(ref src) = export.src {
                        let module_path = format!("{}", src.value);
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
                    Decl::Var(var) => {
                        self.exports
                            .push(ExportRecord::VarDecl { var: var.clone() });
                    }
                    _ => {}
                },
                _ => {
                    //println!("unhandled node: {:#?}", decl);
                }
            },
            _ => {}
        }
    }
}
