//! Builder for creating bundles.

use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{bail, Context, Result};

use swc_common::{FileName, SourceMap, DUMMY_SP};
use swc_ecma_ast::*;
use swc_ecma_loader::{resolve::Resolve, resolvers::node::NodeModulesResolver};

use serde::Serialize;

use crate::{
    module::base::module_base_directory,
    policy::{Merge, Policy},
    swc_utils::{load_file, load_code, get_parser},
};

use super::serializer::{Serializer, Value};

const POLICY_VAR_NAME: &str = "__policy__";
const RUNTIME_PACKAGE: &str = "@lavamoat/lavapack";
//const RUNTIME_FILE: &str = "src/runtime.js";

pub(crate) struct BundleBuilder {
    policy: Policy,
    program: Program,
    source_map: Arc<SourceMap>,
    resolver: Box<dyn Resolve>,
}

impl BundleBuilder {
    /// Create a bundle builder.
    pub fn new() -> Self {
        //let program = Program::Script(Script {
            //span: DUMMY_SP,
            //body: vec![],
            //shebang: None,
        //});

        let source_map: Arc<SourceMap> = Arc::new(Default::default());
        let fm = source_map.new_source_file(
            FileName::Anon,
            "".into(),
        );
        let mut parser = get_parser(&fm);
        let program = parser.parse_program().unwrap();

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

    /// Inject the IIFE into the program.
    pub fn inject_iife(mut self) -> Self {
        let body = self.body_mut();

        let iife = Stmt::Expr(ExprStmt {
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
                                stmts: vec![],
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
        });

        body.push(iife);

        self
    }

    /// Inject the policy into the IIFE body.
    pub fn inject_policy(mut self) -> Result<Self> {
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
                init: Some(Box::new(self.build_policy_object()?)),
            }],
        }));

        {
            let iife = self.iife_mut();
            iife.push(decl);
        }

        Ok(self)
    }

    fn build_policy_object(&self) -> Result<Expr> {
        let mut serializer = Serializer {};
        let value = self.policy.serialize(&mut serializer)?;
        if let Value::Object(obj) = value {
            return Ok(Expr::Object(obj));
        }
        unreachable!("serialized policy must be an object");
    }

    pub fn inject_runtime(mut self) -> Result<Self> {
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

        let iife = self.iife_mut();
        for item in module.body {
            match item {
                ModuleItem::Stmt(stmt) => {
                    iife.push(stmt);
                }
                _ => {}
            }
        }

        Ok(self)
    }

    /// Body of the IIFE.
    ///
    /// Panics if `inject_iife()` has not been invoked yet.
    fn iife_mut(&mut self) -> &mut Vec<Stmt> {
        let body = self.body_mut();
        let iife_node = body.get_mut(0).unwrap();

        if let Stmt::Expr(ExprStmt { expr, .. }) = iife_node {
            if let Expr::Unary(UnaryExpr { arg, .. }) = &mut **expr {
                if let Expr::Call(CallExpr {
                    callee: ExprOrSuper::Expr(expr),
                    ..
                }) = &mut **arg
                {
                    if let Expr::Fn(FnExpr {
                        function:
                            Function {
                                body: Some(body), ..
                            },
                        ..
                    }) = &mut **expr
                    {
                        return &mut body.stmts;
                    }
                }
            }
        }

        unreachable!("Unable to match on IIFE block statement!")
    }

    /// Main body of the program.
    fn body_mut(&mut self) -> &mut Vec<Stmt> {
        if let Program::Script(script) = &mut self.program {
            return &mut script.body;
        }
        unreachable!("Program is not a script!")
    }

    /// Finalize the bundled program.
    pub fn finalize(self) -> Program {
        self.program
    }
}
