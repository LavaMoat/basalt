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

impl ImportRecord {
    pub fn word(&self) -> String {
        match self {
            ImportRecord::StarAs { .. } => String::from("*"),
            ImportRecord::Default { .. } => String::from("default"),
            ImportRecord::Named { local, alias } => alias.clone().unwrap_or(local.clone()),
        }
    }
}

#[derive(Debug)]
pub enum ExportRecord {
    All {
        module_path: String
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
}

impl Visit for ExportAnalysis {
    fn visit_export_all(
        &mut self,
        n: &ExportAll,
        _: &dyn Node
    ) {
        println!("Got export all {:#?}", n);
        let module_path = format!("{}", n.src.value);
        self.exports.push(ExportRecord::All { module_path });
    }

    fn visit_export_decl(
        &mut self,
        n: &ExportDecl,
        _: &dyn Node
    ) {
        //println!("Export DECL {:#?}", n);
    }
}
