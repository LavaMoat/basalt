//! Builder for creating bundles.

use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use anyhow::{Context, Result};

use swc_common::DUMMY_SP;
use swc_ecma_ast::*;

use crate::policy::{Merge, Policy};

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
                    callee: ExprOrSuper::Expr(Box::new(Expr::Member(MemberExpr {
                        span: DUMMY_SP,
                        computed: false,
                        obj: ExprOrSuper::Expr(Box::new(Expr::Fn(FnExpr {
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
                            }
                        }))),
                        prop: Box::new(Expr::Ident(Ident {
                            span: DUMMY_SP,
                            sym: "call".into(),
                            optional: false,
                        }))
                    }))),
                    args: vec![
                        ExprOrSpread {
                            spread: None,
                            expr: Box::new(Expr::This(ThisExpr {span: DUMMY_SP}))
                        }
                    ],
                    type_args: None,

                }))
            }))
        });

        body.push(iife);

        self
    }

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
