//! Generator the functor program from a static module record meta data.
use anyhow::Result;

use swc_atoms::JsWord;
use swc_common::DUMMY_SP;
use swc_ecma_ast::*;

use super::StaticModuleRecord;

const HIDDEN_PREFIX: &str = "$h\u{200d}_";
const HIDDEN_CONST_VAR_PREFIX: &str = "$c\u{200d}_";
const IMPORTS: &str = "imports";
const LIVE_VAR: &str = "liveVar";
const ONCE_VAR: &str = "onceVar";
const MAP: &str = "Map";
const LIVE: &str = "live";

/// Generate a static module record functor program.
pub struct Generator<'a> {
    meta: &'a StaticModuleRecord,
}

impl<'a> Generator<'a> {
    /// Create a new generator.
    pub fn new(meta: &'a StaticModuleRecord) -> Self {
        Generator { meta }
    }

    /// Create the program script AST node.
    pub fn create(&self) -> Result<Script> {
        //println!("{:#?}", self.meta);

        let mut script = Script {
            span: DUMMY_SP,
            body: Vec::with_capacity(1),
            shebang: None,
        };

        let stmt = Stmt::Expr(ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(Expr::Paren(ParenExpr {
                span: DUMMY_SP,
                expr: Box::new(Expr::Arrow(ArrowExpr {
                    span: DUMMY_SP,
                    params: self.params(),
                    body: BlockStmtOrExpr::BlockStmt(self.body()),
                    is_async: false,
                    is_generator: false,
                    type_params: None,
                    return_type: None,
                })),
            })),
        });

        script.body.push(stmt);

        Ok(script)
    }

    /// Build up the functor function parameters.
    fn params(&self) -> Vec<Pat> {
        let props = &[IMPORTS, LIVE_VAR, ONCE_VAR];
        vec![Pat::Object(ObjectPat {
            span: DUMMY_SP,
            props: {
                let mut out = Vec::with_capacity(3);
                for prop in props {
                    out.push(ObjectPatProp::KeyValue(KeyValuePatProp {
                        key: PropName::Ident(Ident {
                            span: DUMMY_SP,
                            sym: (*prop).into(),
                            optional: false,
                        }),
                        value: Box::new(Pat::Ident(BindingIdent {
                            id: Ident {
                                span: DUMMY_SP,
                                sym: self.prefix_hidden(prop),
                                optional: false,
                            },
                            type_ann: None,
                        })),
                    }));
                }
                out
            },
            optional: false,
            type_ann: None,
        })]
    }

    /// The function body block.
    fn body(&self) -> BlockStmt {
        let mut block = BlockStmt {
            span: DUMMY_SP,
            stmts: Vec::new(),
        };

        let local_vars = Stmt::Decl(Decl::Var(VarDecl {
            span: DUMMY_SP,
            kind: VarDeclKind::Let,
            declare: false,
            decls: {
                let mut out = Vec::with_capacity(self.meta.import_decls.len());
                for name in self.meta.import_decls.iter() {
                    let nm: &str = &name[..];
                    out.push(VarDeclarator {
                        span: DUMMY_SP,
                        definite: false,
                        init: None,
                        name: Pat::Ident(BindingIdent {
                            id: Ident {
                                span: DUMMY_SP,
                                sym: nm.into(),
                                optional: false,
                            },
                            type_ann: None,
                        }),
                    });
                }
                out
            },
        }));

        block.stmts.push(local_vars);
        block.stmts.push(self.imports_func_call());

        //block.stmts.push(self.imports_func_call());

        block
    }

    fn imports_func_call(&self) -> Stmt {
        let stmt = Stmt::Expr(ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(Expr::Call(CallExpr {
                span: DUMMY_SP,
                callee: ExprOrSuper::Expr(Box::new(Expr::Ident(Ident {
                    span: DUMMY_SP,
                    sym: self.prefix_hidden(IMPORTS),
                    optional: false,
                }))),
                args: vec![self.imports_arg_map(), self.imports_arg_all()],
                type_args: None,
            })),
        });
        stmt
    }

    /// The arguments passed to the call to orchestrate the imports.
    fn imports_arg_map(&self) -> ExprOrSpread {
        ExprOrSpread {
            spread: None,
            expr: Box::new(Expr::New(NewExpr {
                span: DUMMY_SP,
                callee: Box::new(Expr::Ident(Ident {
                    span: DUMMY_SP,
                    sym: MAP.into(),
                    optional: false,
                })),
                args: Some(self.imports_map_constructor_args()),
                type_args: None,
            })),
        }
    }

    /// The arguments passed to the Map constructor for the first argument
    /// to the call to the imports function.
    fn imports_map_constructor_args(&self) -> Vec<ExprOrSpread> {
        let mut out = Vec::with_capacity(self.meta.imports.len());
        for (key, props) in self.meta.imports.iter() {
            let key: &str = &key[..];
            let aliases = self.meta.import_alias.get(key).unwrap();
            out.push(ExprOrSpread {
                spread: None,
                expr: Box::new(Expr::Array(ArrayLit {
                    span: DUMMY_SP,
                    elems: vec![
                        Some(ExprOrSpread {
                            spread: None,
                            expr: Box::new(Expr::Lit(Lit::Str(Str {
                                span: DUMMY_SP,
                                kind: StrKind::Normal {
                                    contains_quote: true,
                                },
                                value: key.into(),
                                has_escape: false,
                            }))),
                        }),
                        Some(
                            self.imports_map_constructor_args_map(
                                props, aliases,
                            ),
                        ),
                    ],
                })),
            });
        }
        out
    }

    /// The arguments for each nested map.
    fn imports_map_constructor_args_map(
        &self,
        props: &Vec<String>,
        aliases: &Vec<String>,
    ) -> ExprOrSpread {
        ExprOrSpread {
            spread: None,
            expr: Box::new(Expr::New(NewExpr {
                span: DUMMY_SP,
                callee: Box::new(Expr::Ident(Ident {
                    span: DUMMY_SP,
                    sym: MAP.into(),
                    optional: false,
                })),
                args: Some(vec![ExprOrSpread {
                    spread: None,
                    expr: Box::new(Expr::Array(ArrayLit {
                        span: DUMMY_SP,
                        elems: {
                            let mut out = Vec::with_capacity(props.len());
                            for (prop, alias) in
                                props.iter().zip(aliases.iter())
                            {
                                let prop: &str = &prop[..];
                                let alias: &str = &alias[..];
                                let live = self
                                    .meta
                                    .live_export_map
                                    .contains_key(prop);
                                //println!("is live {:?} {:?}", live, prop);

                                out.push(Some(ExprOrSpread {
                                    spread: None,
                                    expr: Box::new(Expr::Array(ArrayLit {
                                        span: DUMMY_SP,
                                        elems: vec![
                                            Some(ExprOrSpread {
                                                spread: None,
                                                expr: Box::new(Expr::Lit(
                                                    Lit::Str(Str {
                                                        span: DUMMY_SP,
                                                        kind: StrKind::Normal {
                                                            contains_quote:
                                                                true,
                                                        },
                                                        value: prop.into(),
                                                        has_escape: false,
                                                    }),
                                                )),
                                            }),
                                            Some(ExprOrSpread {
                                                spread: None,
                                                expr: Box::new(Expr::Array(
                                                    ArrayLit {
                                                        span: DUMMY_SP,
                                                        elems: vec![Some(self.imports_prop_func(alias, live))],
                                                    },
                                                )),
                                            }),
                                        ],
                                    })),
                                }));
                            }
                            out
                        },
                    })),
                }]),
                type_args: None,
            })),
        }
    }

    /// The import function which lazily assigns to the locally scoped variable.
    fn imports_prop_func(&self, alias: &str, live: bool) -> ExprOrSpread {
        let arg = self.prefix_hidden("a");
        if live {
            ExprOrSpread {
                spread: None,
                expr: Box::new(Expr::Member(MemberExpr {
                    span: DUMMY_SP,
                    obj: ExprOrSuper::Expr(Box::new(Expr::Ident(Ident {
                        span: DUMMY_SP,
                        sym: self.prefix_hidden(LIVE),
                        optional: false,
                    }))),
                    prop: Box::new(Expr::Lit(Lit::Str(Str {
                        span: DUMMY_SP,
                        kind: StrKind::Normal {
                            contains_quote: true,
                        },
                        has_escape: false,
                        value: alias.into(),
                    }))),
                    computed: true,
                })),
            }
        } else {
            ExprOrSpread {
                spread: None,
                expr: Box::new(Expr::Arrow(ArrowExpr {
                    span: DUMMY_SP,
                    params: vec![Pat::Ident(BindingIdent {
                        id: Ident {
                            span: DUMMY_SP,
                            sym: arg.clone(),
                            optional: false,
                        },
                        type_ann: None,
                    })],
                    body: BlockStmtOrExpr::Expr(Box::new(Expr::Paren(
                        ParenExpr {
                            span: DUMMY_SP,
                            expr: Box::new(Expr::Assign(AssignExpr {
                                span: DUMMY_SP,
                                op: AssignOp::Assign,
                                left: PatOrExpr::Pat(Box::new(Pat::Ident(
                                    BindingIdent {
                                        id: Ident {
                                            span: DUMMY_SP,
                                            sym: alias.into(),
                                            optional: false,
                                        },
                                        type_ann: None,
                                    },
                                ))),
                                right: Box::new(Expr::Ident(Ident {
                                    span: DUMMY_SP,
                                    sym: arg,
                                    optional: false,
                                })),
                            })),
                        },
                    ))),
                    is_async: false,
                    is_generator: false,
                    return_type: None,
                    type_params: None,
                })),
            }
        }
    }

    /// All imports argument. The second parameter when invoking the
    /// imports orchestration function.
    fn imports_arg_all(&self) -> ExprOrSpread {
        ExprOrSpread {
            spread: None,
            expr: Box::new(Expr::Array(ArrayLit {
                span: DUMMY_SP,
                elems: {
                    let mut out =
                        Vec::with_capacity(self.meta.export_alls.len());
                    for name in self.meta.export_alls.iter() {
                        let nm: &str = &name[..];
                        out.push(Some(ExprOrSpread {
                            spread: None,
                            expr: Box::new(Expr::Lit(Lit::Str(Str {
                                span: DUMMY_SP,
                                kind: StrKind::Normal {
                                    contains_quote: true,
                                },
                                value: nm.into(),
                                has_escape: false,
                            }))),
                        }));
                    }
                    out
                },
            })),
        }
    }

    fn prefix_hidden(&self, word: &str) -> JsWord {
        format!("{}{}", HIDDEN_PREFIX, word).into()
    }

    fn prefix_const(&self, word: &str) -> JsWord {
        format!("{}{}", HIDDEN_CONST_VAR_PREFIX, word).into()
    }
}
