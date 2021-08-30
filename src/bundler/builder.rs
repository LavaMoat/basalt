//! Builder for creating bundles.

use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{bail, Context, Result};

use swc_common::{FileName, SourceMap, DUMMY_SP};
use swc_ecma_ast::*;
use swc_ecma_loader::{resolve::Resolve, resolvers::node::NodeModulesResolver};
use swc_ecma_visit::{Fold, FoldWith};

use serde::Serialize;

use crate::{
    module::base::module_base_directory,
    policy::{Merge, Policy},
    swc_utils::{get_parser, load_file},
};

use super::serializer::{Serializer, Value};

const POLICY_VAR_NAME: &str = "__policy__";
const RUNTIME_PACKAGE: &str = "@lavamoat/lavapack";

pub(crate) struct BundleBuilder {
    policy: Policy,
    program: Program,
    source_map: Arc<SourceMap>,
    resolver: Box<dyn Resolve>,
}

impl BundleBuilder {
    /// Create a bundle builder.
    pub fn new() -> Self {
        let source_map: Arc<SourceMap> = Arc::new(Default::default());
        let program = Program::Script(Script {
            span: DUMMY_SP,
            body: vec![],
            shebang: None,
        });
        let resolver: Box<dyn Resolve> =
            Box::new(NodeModulesResolver::default());

        Self {
            policy: Default::default(),
            program,
            source_map,
            resolver,
        }
    }

    /// Load policy files.
    pub fn load_policy_files(mut self, policy: &Vec<PathBuf>) -> Result<Self> {
        for file in policy {
            let f = File::open(file).context(format!(
                "Unable to open policy file {}",
                file.display()
            ))?;
            let reader = BufReader::new(f);
            let mut policy: Policy = serde_json::from_reader(reader).context(
                format!("Failed to parse JSON in {}", file.display()),
            )?;
            self.policy.merge(&mut policy);
        }

        Ok(self)
    }

    /// Fold into a single program.
    pub fn fold(mut self) -> Result<Self> {
        let policy_expr = PolicyDecl::build_policy(std::mem::take(&mut self.policy))?;
        let mut policy_decl = PolicyDecl { expr: policy_expr };
        self.program = self.program.fold_with(&mut policy_decl);

        let module = self.load_runtime_module()?;
        let mut runtime_module = RuntimeModule { module };
        self.program = self.program.fold_with(&mut runtime_module);

        let mut iife = Iife {};
        self.program = self.program.fold_with(&mut iife);

        Ok(self)
    }

    fn load_runtime_module(&self) -> Result<Module> {
        let base_dir = FileName::Real(std::env::current_dir()?);
        let runtime_lib = self
            .resolver
            .resolve(&base_dir, RUNTIME_PACKAGE)
            .context(format!(
                "could not find {}, ensure it has been installed",
                RUNTIME_PACKAGE
            ))?;

        let lib_index = match runtime_lib {
            FileName::Real(path) => path,
            _ => bail!("runtime library must be a real path"),
        };
        let package_dir = module_base_directory(&lib_index).unwrap();
        let runtime_file = package_dir.join("src").join("runtime.js");

        if !runtime_file.is_file() {
            bail!("runtime {} is not a file", runtime_file.display());
        }

        let (_, _, module) =
            load_file(&runtime_file, Some(Arc::clone(&self.source_map)))?;

        Ok(module)
    }

    /// Finalize the bundled program.
    pub fn finalize(self) -> (Program, Arc<SourceMap>) {
        (self.program, self.source_map)
    }
}

struct PolicyDecl {
    expr: Expr,
}

impl PolicyDecl {
    fn build_policy(policy: Policy) -> Result<Expr> {
        let mut serializer = Serializer {};
        let value = policy.serialize(&mut serializer)?;
        if let Value::Object(obj) = value {
            return Ok(Expr::Object(obj));
        }
        unreachable!("serialized policy must be an object");
    }
}

impl Fold for PolicyDecl {
    fn fold_program(&mut self, mut n: Program) -> Program {
        let decl = Stmt::Decl(Decl::Var(VarDecl {
            span: DUMMY_SP,
            kind: VarDeclKind::Var,
            declare: false,
            decls: vec![VarDeclarator {
                span: DUMMY_SP,
                definite: false,
                name: Pat::Ident(BindingIdent {
                    id: Ident {
                        span: DUMMY_SP,
                        optional: false,
                        sym: POLICY_VAR_NAME.into(),
                    },
                    type_ann: None,
                }),
                init: Some(Box::new(self.expr.clone())),
            }],
        }));

        let stmts = if let Program::Script(script) = &mut n {
            &mut script.body
        } else {
            panic!("Expecting script program, got a module")
        };

        stmts.push(decl);

        n
    }
}


struct RuntimeModule {
    module: Module,
}

impl Fold for RuntimeModule {
    fn fold_program(&mut self, mut n: Program) -> Program {
        let stmts = if let Program::Script(script) = &mut n {
            &mut script.body
        } else {
            panic!("Expecting script program, got a module")
        };

        let mut module_stmts: Vec<Stmt> = self.module.body
            .iter()
            .filter(|item| match item {
                ModuleItem::Stmt(_) => true,
                _ => false,
            })
            .map(|item| match item {
                ModuleItem::Stmt(stmt) => stmt.clone(),
                _ => unreachable!(),
            })
            .collect();

        stmts.append(&mut module_stmts);

        n
    }
}

struct Iife;
impl Fold for Iife {
    fn fold_program(&mut self, n: Program) -> Program {
        let stmts = if let Program::Script(script) = n {
            script.body
        } else {
            panic!("Expecting script program, got a module")
        };

        Program::Script(Script {
            span: DUMMY_SP,
            body: vec![Stmt::Expr(ExprStmt {
                span: DUMMY_SP,
                expr: Box::new(Expr::Unary(UnaryExpr {
                    span: DUMMY_SP,
                    op: UnaryOp::Void,
                    arg: Box::new(Expr::Call(CallExpr {
                        span: DUMMY_SP,
                        callee: ExprOrSuper::Expr(Box::new(Expr::Fn(FnExpr {
                            ident: None,
                            function: Function {
                                params: vec![],
                                body: Some(BlockStmt {
                                    span: DUMMY_SP,
                                    stmts,
                                }),
                                decorators: vec![],
                                span: DUMMY_SP,
                                is_generator: false,
                                is_async: false,
                                type_params: None,
                                return_type: None,
                            },
                        }))),
                        args: vec![],
                        type_args: None,
                    })),
                })),
            })],
            shebang: None,
        })
    }
}
