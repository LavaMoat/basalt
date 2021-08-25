//! Builder for creating bundles.

use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use anyhow::{Context, Result};

use swc_common::DUMMY_SP;
use swc_ecma_ast::*;

use serde::Serialize;

use crate::policy::{Merge, Policy};

use super::serializer::Serializer;

const RESOURCES: &str = "resources";
const POLICY_VAR_NAME: &str = "__policy__";
const RUNTIME_PACKAGE: &str = "@lavamoat/lavapack";
const RUNTIME_FILE: &str = "src/runtime.js";

pub(crate) struct BundleBuilder {
    policy: Policy,
    program: Program,
}

impl BundleBuilder {
    /// Create a bundle builder.
    pub fn new() -> Self {
        let program = Program::Script(Script {
            span: DUMMY_SP,
            body: vec![],
            shebang: None,
        });

        Self {
            policy: Default::default(),
            program,
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
    pub fn inject_policy(mut self) -> Self {
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
                init: Some(Box::new(self.build_policy_object())),
            }],
        }));

        {
            let iife = self.iife_mut();
            iife.push(decl);
        }

        self
    }

    fn build_policy_object(&self) -> Expr {
        let mut serializer = Serializer {};
        let result = self.policy.serialize(&mut serializer);

        println!("Got serialize result {:#?}", result);

        Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(
                KeyValueProp {
                    key: PropName::Str(str_lit(RESOURCES)),
                    value: Box::new(Expr::Object(ObjectLit {
                        span: DUMMY_SP,
                        props: {
                            let mut out =
                                Vec::with_capacity(self.policy.resources.len());
                            for (k, v) in self.policy.resources.iter() {}
                            out
                        },
                    })),
                },
            )))],
        })
    }

    //fn build_package_policy_object() -> Expr {

    //}

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

fn str_lit(value: &str) -> Str {
    Str {
        span: DUMMY_SP,
        value: value.into(),
        has_escape: value.contains("\n"),
        kind: StrKind::Normal {
            contains_quote: false,
        },
    }
}
