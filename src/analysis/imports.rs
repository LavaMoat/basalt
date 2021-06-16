//! Helper to analyze imports from a module.
use swc_ecma_ast::*;
use swc_ecma_visit::{Node, Visit};

use indexmap::IndexMap;

#[derive(Debug)]
pub enum ImportRecord {
    None,
    All { name: String },
    Default { name: String },
    Named { name: String, alias: String },
}

#[derive(Default, Debug)]
pub struct ImportAnalysis {
    pub imports: IndexMap<String, Vec<ImportRecord>>,
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
