//! Generator the functor program from a static module record meta data.
use anyhow::Result;

use swc_atoms::JsWord;
use swc_common::DUMMY_SP;
use swc_ecma_ast::*;
use swc_ecma_visit::{Node, Visit, VisitWith};

use indexmap::IndexMap;

use super::{ImportName, StaticModuleRecord};

const HIDDEN_PREFIX: &str = "$h\u{200d}_";
const HIDDEN_CONST_VAR_PREFIX: &str = "$c\u{200d}_";
const IMPORTS: &str = "imports";
const LIVE_VAR: &str = "liveVar";
const ONCE_VAR: &str = "onceVar";
const MAP: &str = "Map";
const LIVE: &str = "live";
const ONCE: &str = "once";
const DEFAULT: &str = "default";

fn prefix_hidden(word: &str) -> JsWord {
    format!("{}{}", HIDDEN_PREFIX, word).into()
}

fn prefix_const(word: &str) -> JsWord {
    format!("{}{}", HIDDEN_CONST_VAR_PREFIX, word).into()
}

struct Visitor<'a> {
    meta: &'a StaticModuleRecord<'a>,
    body: &'a mut Vec<Stmt>,
}

fn var_symbol_names(var: &VarDecl) -> Vec<(&str, &VarDeclarator)> {
    var.decls
        .iter()
        .filter(|decl| match &decl.name {
            Pat::Ident(_binding) => true,
            _ => false,
        })
        .map(|decl| match &decl.name {
            Pat::Ident(binding) => (binding.id.sym.as_ref(), decl),
            _ => unreachable!(),
        })
        .collect::<Vec<_>>()
}

fn call_stmt(prop_target: JsWord, prop_name: &str, arg: JsWord) -> Stmt {
    Stmt::Expr(ExprStmt {
        span: DUMMY_SP,
        expr: Box::new(Expr::Call(CallExpr {
            span: DUMMY_SP,
            callee: ExprOrSuper::Expr(Box::new(Expr::Member(MemberExpr {
                span: DUMMY_SP,
                obj: ExprOrSuper::Expr(Box::new(Expr::Ident(Ident {
                    span: DUMMY_SP,
                    sym: prop_target,
                    optional: false,
                }))),
                prop: Box::new(Expr::Ident(Ident {
                    span: DUMMY_SP,
                    sym: prop_name.into(),
                    optional: false,
                })),
                computed: false,
            }))),
            args: vec![ExprOrSpread {
                spread: None,
                expr: Box::new(Expr::Ident(Ident {
                    span: DUMMY_SP,
                    sym: arg,
                    optional: false,
                })),
            }],
            type_args: None,
        })),
    })
}

impl<'a> Visitor<'a> {
    /// Get a potential symbol identity from a statement.
    fn identity<'b>(&mut self, n: &'b Stmt) -> Option<&'b str> {
        match n {
            Stmt::Expr(expr) => match &*expr.expr {
                Expr::Assign(expr) => match &expr.left {
                    PatOrExpr::Pat(pat) => match &**pat {
                        Pat::Ident(ident) => {
                            return Some(ident.id.sym.as_ref())
                        }
                        _ => {}
                    },
                    _ => {}
                },
                _ => {}
            },
            Stmt::Decl(decl) => match decl {
                Decl::Var(var) => {
                    // TODO: support multiple var declarations
                    if !var.decls.is_empty() {
                        let name = &var.decls.get(0).unwrap().name;
                        match name {
                            Pat::Ident(ident) => {
                                return Some(ident.id.sym.as_ref())
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        }
        None
    }

    fn is_live_statement<'b>(
        &mut self,
        n: &'b Stmt,
    ) -> (bool, Option<&'b str>) {
        if let Some(identity) = self.identity(n) {
            if self.meta.live_export_map.contains_key(identity) {
                return (true, Some(identity));
            }
        }
        (false, None)
    }
}

impl<'a> Visit for Visitor<'a> {
    fn visit_module_item(&mut self, n: &ModuleItem, node: &dyn Node) {
        match n {
            ModuleItem::ModuleDecl(decl) => match decl {
                ModuleDecl::ExportNamed(export) => {
                    // Not a re-export
                    if export.src.is_none() {
                        for spec in export.specifiers.iter() {
                            if let ExportSpecifier::Named(spec) = spec {
                                let export_name = spec
                                    .exported
                                    .as_ref()
                                    .unwrap_or(&spec.orig)
                                    .sym
                                    .as_ref();
                                let local_name = spec.orig.sym.as_ref();
                                let prop_target = prefix_hidden(ONCE);
                                let call = call_stmt(
                                    prop_target,
                                    export_name,
                                    local_name.into(),
                                );
                                self.body.push(call);
                            }
                        }
                    }
                }
                ModuleDecl::ExportDefaultDecl(export) => {
                    // TODO: class declaration exports.
                    //todo!("Export default decl");
                }
                ModuleDecl::ExportDefaultExpr(export) => {
                    // TODO
                    // const { default: $c_default } = { default: 42 };
                    // $h_once.default($c_default);
                    if self.meta.fixed_export_map.contains_key(DEFAULT) {
                        let prop_target = prefix_hidden(ONCE);
                        let prop_arg = prefix_const(DEFAULT);
                        let value_expr = export.expr.clone();

                        let default_stmt = Stmt::Decl(Decl::Var(VarDecl {
                            span: DUMMY_SP,
                            // Default exports must be constant
                            kind: VarDeclKind::Const,
                            declare: false,
                            decls: vec![VarDeclarator {
                                span: DUMMY_SP,
                                definite: false,
                                name: Pat::Object(ObjectPat {
                                    span: DUMMY_SP,
                                    optional: false,
                                    type_ann: None,
                                    props: vec![ObjectPatProp::KeyValue(
                                        KeyValuePatProp {
                                            key: PropName::Ident(Ident {
                                                span: DUMMY_SP,
                                                optional: false,
                                                sym: DEFAULT.into(),
                                            }),
                                            value: Box::new(Pat::Ident(
                                                BindingIdent {
                                                    id: Ident {
                                                        span: DUMMY_SP,
                                                        optional: false,
                                                        sym: prop_arg.clone(),
                                                    },
                                                    type_ann: None,
                                                },
                                            )),
                                        },
                                    )],
                                }),
                                init: Some(Box::new(Expr::Object(ObjectLit {
                                    span: DUMMY_SP,
                                    props: vec![PropOrSpread::Prop(Box::new(
                                        Prop::KeyValue(KeyValueProp {
                                            key: PropName::Ident(Ident {
                                                span: DUMMY_SP,
                                                optional: false,
                                                sym: DEFAULT.into(),
                                            }),
                                            value: value_expr,
                                        }),
                                    ))],
                                }))),
                            }],
                        }));

                        let call = call_stmt(prop_target, "default", prop_arg);

                        self.body.push(default_stmt);
                        self.body.push(call);
                    }
                }
                ModuleDecl::ExportDecl(export) => match &export.decl {
                    Decl::Var(var) => {
                        let names = var_symbol_names(var);
                        for (name, decl) in names {
                            if self.meta.fixed_export_map.contains_key(name) {
                                self.body.push(Stmt::Decl(Decl::Var(
                                    VarDecl {
                                        span: DUMMY_SP,
                                        kind: var.kind.clone(),
                                        declare: false,
                                        decls: vec![decl.clone()],
                                    },
                                )));

                                let prop_target = prefix_hidden(ONCE);
                                // TODO: handle alias in fixed exports!
                                let call =
                                    call_stmt(prop_target, name, name.into());
                                self.body.push(call);
                            }
                        }
                    }
                    _ => {}
                },
                _ => {}
            },
            ModuleItem::Stmt(stmt) => self.visit_stmt(stmt, node),
        }
    }

    fn visit_stmt(&mut self, n: &Stmt, _: &dyn Node) {
        let (is_live, live_name) = self.is_live_statement(n);
        if is_live {
            let live_name = live_name.unwrap();
            let prop_name = prefix_const(live_name);
            let prop_target = prefix_hidden(LIVE);

            let decl = Stmt::Decl(Decl::Var(VarDecl {
                span: DUMMY_SP,
                kind: VarDeclKind::Let,
                declare: false,
                decls: vec![VarDeclarator {
                    span: DUMMY_SP,
                    name: Pat::Ident(BindingIdent {
                        id: Ident {
                            span: DUMMY_SP,
                            sym: prop_name.clone(),
                            optional: false,
                        },
                        type_ann: None,
                    }),
                    // NOTE: currently we always initialize to null
                    // NOTE: an improvement could respect the source
                    // NOTE: initialization value
                    init: Some(Box::new(Expr::Lit(Lit::Null(Null {
                        span: DUMMY_SP,
                    })))),
                    definite: false,
                }],
            }));

            let call = call_stmt(prop_target, live_name, prop_name);

            self.body.push(decl);
            self.body.push(call);

            // TODO: Rename the variable on the left hand side of an assignment???
            self.body.push(n.clone());
        } else {
            self.body.push(n.clone());
        }
    }
}

/// Generate a static module record functor program.
pub struct Generator<'a> {
    meta: &'a StaticModuleRecord<'a>,
}

impl<'a> Generator<'a> {
    /// Create a new generator.
    pub fn new(meta: &'a StaticModuleRecord<'a>) -> Self {
        Generator { meta }
    }

    /// Create the program script AST node.
    pub fn create(&self) -> Result<Script> {
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
        let mut props = IndexMap::new();
        props.insert(IMPORTS, IMPORTS);
        props.insert(LIVE_VAR, LIVE);
        props.insert(ONCE_VAR, ONCE);
        vec![Pat::Object(ObjectPat {
            span: DUMMY_SP,
            props: {
                let mut out = Vec::with_capacity(props.len());
                for (prop, target) in props {
                    out.push(ObjectPatProp::KeyValue(KeyValuePatProp {
                        key: PropName::Ident(Ident {
                            span: DUMMY_SP,
                            sym: (*prop).into(),
                            optional: false,
                        }),
                        value: Box::new(Pat::Ident(BindingIdent {
                            id: Ident {
                                span: DUMMY_SP,
                                sym: prefix_hidden(target),
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

        let decls = self.meta.decls();

        if !decls.is_empty() {
            let local_vars = Stmt::Decl(Decl::Var(VarDecl {
                span: DUMMY_SP,
                kind: VarDeclKind::Let,
                declare: false,
                decls: {
                    let mut out = Vec::with_capacity(decls.len());
                    for name in decls.iter() {
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
        }

        block.stmts.push(self.imports_func_call());

        let mut visitor = Visitor {
            meta: self.meta,
            body: &mut block.stmts,
        };
        self.meta.module.visit_children_with(&mut visitor);

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
                    sym: prefix_hidden(IMPORTS),
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
        vec![
            ExprOrSpread {
                spread: None,
                expr: Box::new(Expr::Array(ArrayLit {
                    span: DUMMY_SP,
                    elems: {
                        let mut out = Vec::with_capacity(self.meta.imports.len());
                        for (key, props) in self.meta.imports.iter() {
                            let key: &str = &key[..];
                            let computed_aliases = self.meta.aliases();
                            let aliases = computed_aliases.get(key).unwrap();
                            out.push(Some(ExprOrSpread {
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
                            }));
                        }
                        out
                    }
                })),
            }
        ]
    }

    /// The arguments for each nested map.
    fn imports_map_constructor_args_map(
        &self,
        props: &Vec<ImportName<'a>>,
        aliases: &Vec<&str>,
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
                                let prop = prop.raw_name();
                                let alias: &str = &alias[..];
                                let live = self
                                    .meta
                                    .live_export_map
                                    .contains_key(prop)
                                    || {
                                        // NOTE: This is a bit of a hack :(
                                        // NOTE: becuase imports doesn't contain the local name
                                        // NOTE: so `export {gray as grey} from './gray.js'` only
                                        // NOTE: gives us `grey` and `grey` right now.
                                        let live_alias = self
                                            .meta
                                            .live_export_map
                                            .iter()
                                            .find(|(_k, v)| {
                                                if v.0 == prop {
                                                    return true;
                                                }
                                                false
                                            });

                                        live_alias.is_some()
                                    };

                                //println!("is live {:?} {:?} {:?}", live, prop, alias);

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
        let arg = prefix_hidden("a");
        if live {
            ExprOrSpread {
                spread: None,
                expr: Box::new(Expr::Member(MemberExpr {
                    span: DUMMY_SP,
                    obj: ExprOrSuper::Expr(Box::new(Expr::Ident(Ident {
                        span: DUMMY_SP,
                        sym: prefix_hidden(LIVE),
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
}
