//! Helper to analyze exports from a module.
use swc_ecma_ast::*;
use swc_ecma_visit::{Node, Visit};

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

    fn check_live_statement(&mut self, stmt: &Stmt) {

        println!("Check live statement {:#?}", stmt);

        match stmt {
            // Track assignments for live export map.
            //
            // NOTE: this currently only handles assignments at the module level.
            Stmt::Expr(expr) => match &*expr.expr {
                Expr::Assign(expr) => {
                    match &expr.left {
                        PatOrExpr::Pat(pat) => match &**pat {
                            Pat::Ident(ident) => {
                                let lhs = format!("{}", ident.id.sym);
                                // Set if we can find an existing export that would
                                // receive the assignment.
                                for rec in self.exports.iter() {
                                    match rec {
                                        ExportRecord::VarDecl { var } => {
                                            for decl in var.decls.iter() {
                                                match &decl.name {
                                                    Pat::Ident(ident) => {
                                                        let target_name = format!("{}", ident.id.sym);
                                                        if lhs == target_name {
                                                            self.live.push(target_name);
                                                            break;
                                                        }
                                                    }
                                                    _ => {}
                                                }
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            _ => {}
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
            _ => {}
        }
    }
}

impl Visit for ExportAnalysis {
    fn visit_module_item(&mut self, n: &ModuleItem, _: &dyn Node) {
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
                ModuleDecl::ExportDefaultExpr(export) => {
                    self.exports.push(ExportRecord::DefaultExpr {
                        expr: export.expr.clone(),
                    });
                }
                _ => {
                    //println!("unhandled node: {:#?}", decl);
                }
            }
            ModuleItem::Stmt(stmt) => self.check_live_statement(stmt)
        }
    }

}
