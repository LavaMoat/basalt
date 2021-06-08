//! Helper to analyze imports from a module.
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
