//! Helper to analyize imports and exports from a module
use std::collections::HashMap;
use swc_ecma_ast::*;
use swc_ecma_visit::{Node, Visit};

#[derive(Debug)]
pub enum ImportRecord {
    StarAs {
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

#[derive(Debug)]
pub enum ExportRecord {}

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

    pub fn analyze(&mut self, module: Module) {}
}

impl Visit for ImportAnalysis {
    fn visit_import_decl(&mut self, n: &ImportDecl, _parent: &dyn Node) {
        let module_path = format!("{}", n.src.value);
        for spec in n.specifiers.iter() {
            let mut list = self
                .imports
                .entry(module_path.clone())
                .or_insert(Vec::new());
            match spec {
                ImportSpecifier::Namespace(item) => {
                    list.push(ImportRecord::StarAs {
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
}

impl ExportAnalysis {
    pub fn new() -> Self {
        Self {
            exports: Default::default(),
        }
    }

    pub fn analyze(&mut self, module: Module) {}
}

impl Visit for ExportAnalysis {
    // TODO
}
