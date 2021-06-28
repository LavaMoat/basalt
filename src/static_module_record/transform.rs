//! Transform a module to a static module record program.
use std::path::PathBuf;
use std::sync::Arc;

use indexmap::IndexMap;

use swc::{
    config::{JscTarget, Options, SourceMapsConfig},
    Compiler, TransformOutput,
};
use swc_common::{
    errors::{emitter::ColorConfig, Handler},
    FileName, SourceMap, DUMMY_SP,
};

use swc_atoms::JsWord;
use swc_ecma_ast::*;
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax};
use swc_ecma_visit::{Node, Visit, VisitWith};

use anyhow::{Context, Result};

use super::{
    parser::var_symbol_names, ImportName, Parser as StaticModuleRecordParser,
    StaticModuleRecord, StaticModuleRecordMeta,
};

const HIDDEN_PREFIX: &str = "$h\u{200d}_";
const HIDDEN_CONST_VAR_PREFIX: &str = "$c\u{200d}_";
const IMPORTS: &str = "imports";
const LIVE_VAR: &str = "liveVar";
const ONCE_VAR: &str = "onceVar";
const MAP: &str = "Map";
const LIVE: &str = "live";
const ONCE: &str = "once";
const DEFAULT: &str = "default";
const OBJECT: &str = "Object";
const DEFINE_PROPERTY: &str = "defineProperty";
const NAME: &str = "name";
const VALUE: &str = "value";

fn prefix_hidden(word: &str) -> JsWord {
    format!("{}{}", HIDDEN_PREFIX, word).into()
}

fn prefix_const(word: &str) -> JsWord {
    format!("{}{}", HIDDEN_CONST_VAR_PREFIX, word).into()
}

/// Sources that may be transformed
#[derive(Debug)]
pub enum TransformSource {
    /// Load a file from disc for the transformation.
    File(PathBuf),
    /// Load from a string.
    Str {
        /// The module source.
        content: String,
        /// The file name for the module.
        file_name: String,
    },
}

/// Result of parsing a source module.
pub struct ParseOutput<'a> {
    /// The source map.
    pub source_map: Arc<SourceMap>,
    /// The parser stores the analyzer and underlying
    /// references.
    pub parser: StaticModuleRecordParser,
    /// The parsed source module AST.
    pub module: Module,
    /// The computed static module record meta data.
    pub meta: StaticModuleRecord<'a>,
}

/// Transform the module file to a program script.
pub fn transform(
    source: TransformSource,
) -> Result<(StaticModuleRecordMeta, TransformOutput)> {
    let sm: Arc<SourceMap> = Arc::new(Default::default());
    let handler = Handler::with_tty_emitter(
        ColorConfig::Auto,
        true,
        false,
        Some(sm.clone()),
    );

    let mut options: Options = Default::default();
    options.source_maps = Some(SourceMapsConfig::Bool(true));

    let fm = match source {
        TransformSource::File(path) => sm.load_file(&path)?,
        TransformSource::Str { content, file_name } => sm.new_source_file(
            FileName::Custom(file_name.into()),
            content.into(),
        ),
    };

    let lexer = Lexer::new(
        Syntax::Es(Default::default()),
        JscTarget::Es2020,
        StringInput::from(&*fm),
        None,
    );

    let mut parser = Parser::new_from(lexer);
    for e in parser.take_errors() {
        e.into_diagnostic(&handler).emit();
    }

    let module = parser
        .parse_module()
        .map_err(|e| e.into_diagnostic(&handler).emit())
        .expect("failed to parse module");

    let mut parser = StaticModuleRecordParser::new();
    let meta = parser.parse(&module)?;

    let generator = Generator::new(&meta);
    let compiler = Compiler::new(sm, Arc::new(handler));
    let script = generator
        .create()
        .context("failed to generate transformed script")?;
    let program = Program::Script(script);

    let result = compiler.print(
        &program,
        JscTarget::Es2020,
        SourceMapsConfig::Bool(true),
        None,
        false,
    )?;

    Ok((meta.into(), result))
}

struct Visitor<'a> {
    meta: &'a StaticModuleRecord<'a>,
    body: &'a mut Vec<Stmt>,
}

fn call_stmt(
    prop_target: JsWord,
    prop_name: &str,
    mut arg: Option<JsWord>,
) -> Stmt {
    let args = if let Some(arg) = arg.take() {
        vec![ExprOrSpread {
            spread: None,
            expr: Box::new(Expr::Ident(Ident {
                span: DUMMY_SP,
                sym: arg,
                optional: false,
            })),
        }]
    } else {
        vec![]
    };

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
            args,
            type_args: None,
        })),
    })
}

fn default_stmt(
    prop_target: JsWord,
    prop_arg: JsWord,
    value: Box<Expr>,
) -> (Stmt, Stmt) {
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
                props: vec![ObjectPatProp::KeyValue(KeyValuePatProp {
                    key: PropName::Ident(Ident {
                        span: DUMMY_SP,
                        optional: false,
                        sym: DEFAULT.into(),
                    }),
                    value: Box::new(Pat::Ident(BindingIdent {
                        id: Ident {
                            span: DUMMY_SP,
                            optional: false,
                            sym: prop_arg.clone(),
                        },
                        type_ann: None,
                    })),
                })],
            }),
            init: Some(Box::new(Expr::Object(ObjectLit {
                span: DUMMY_SP,
                props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(
                    KeyValueProp {
                        key: PropName::Ident(Ident {
                            span: DUMMY_SP,
                            optional: false,
                            sym: DEFAULT.into(),
                        }),
                        value,
                    },
                )))],
            }))),
        }],
    }));

    (
        default_stmt,
        call_stmt(prop_target, "default", Some(prop_arg)),
    )
}

fn define_property(target: &str, prop_name: &str, prop_value: &str) -> Stmt {
    Stmt::Expr(ExprStmt {
        span: DUMMY_SP,
        expr: Box::new(Expr::Call(CallExpr {
            span: DUMMY_SP,
            callee: ExprOrSuper::Expr(Box::new(Expr::Member(MemberExpr {
                span: DUMMY_SP,
                obj: ExprOrSuper::Expr(Box::new(Expr::Ident(Ident {
                    span: DUMMY_SP,
                    sym: OBJECT.into(),
                    optional: false,
                }))),
                prop: Box::new(Expr::Ident(Ident {
                    span: DUMMY_SP,
                    sym: DEFINE_PROPERTY.into(),
                    optional: false,
                })),
                computed: false,
            }))),
            args: vec![
                ExprOrSpread {
                    spread: None,
                    expr: Box::new(Expr::Ident(Ident {
                        span: DUMMY_SP,
                        sym: target.into(),
                        optional: false,
                    })),
                },
                ExprOrSpread {
                    spread: None,
                    expr: Box::new(Expr::Lit(Lit::Str(Str {
                        span: DUMMY_SP,
                        kind: StrKind::Normal {
                            contains_quote: true,
                        },
                        value: prop_name.into(),
                        has_escape: false,
                    }))),
                },
                ExprOrSpread {
                    spread: None,
                    expr: Box::new(Expr::Object(ObjectLit {
                        span: DUMMY_SP,
                        props: vec![PropOrSpread::Prop(Box::new(
                            Prop::KeyValue(KeyValueProp {
                                key: PropName::Ident(Ident {
                                    span: DUMMY_SP,
                                    sym: VALUE.into(),
                                    optional: false,
                                }),
                                value: Box::new(Expr::Lit(Lit::Str(Str {
                                    span: DUMMY_SP,
                                    kind: StrKind::Normal {
                                        contains_quote: true,
                                    },
                                    value: prop_value.into(),
                                    has_escape: false,
                                }))),
                            }),
                        ))],
                    })),
                },
            ],
            type_args: None,
        })),
    })
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
                                    Some(local_name.into()),
                                );
                                self.body.push(call);
                            }
                        }
                    }
                }
                ModuleDecl::ExportDefaultDecl(export) => {
                    let prop_target = prefix_hidden(ONCE);
                    let prop_arg = prefix_const(DEFAULT);
                    let value_expr = match &export.decl {
                        DefaultDecl::Class(class_expr) => {
                            Box::new(Expr::Class(class_expr.clone()))
                        }
                        DefaultDecl::Fn(fn_expr) => {
                            Box::new(Expr::Fn(fn_expr.clone()))
                        }
                        _ => panic!("Typescript interface declarations are not supported")
                    };
                    let (default_stmt, call) =
                        default_stmt(prop_target, prop_arg, value_expr);
                    self.body.push(default_stmt);
                    self.body.push(call);
                }
                ModuleDecl::ExportDefaultExpr(export) => {
                    // const { default: $c_default } = { default: 42 };
                    // $h_once.default($c_default);
                    if self.meta.fixed_export_map.contains_key(DEFAULT) {
                        let prop_target = prefix_hidden(ONCE);
                        let prop_arg = prefix_const(DEFAULT);
                        let value_expr = export.expr.clone();
                        let (default_stmt, call) =
                            default_stmt(prop_target, prop_arg, value_expr);
                        self.body.push(default_stmt);
                        self.body.push(call);
                    }
                }
                ModuleDecl::ExportDecl(export) => match &export.decl {
                    Decl::Var(var) => {
                        let symbols = var_symbol_names(var);
                        for (decl, names) in symbols {
                            let mut decl_emitted = false;
                            for name in names {
                                if self.meta.fixed_export_map.contains_key(name)
                                {
                                    if !decl_emitted {
                                        self.body.push(Stmt::Decl(Decl::Var(
                                            VarDecl {
                                                span: DUMMY_SP,
                                                kind: var.kind.clone(),
                                                declare: false,
                                                decls: vec![decl.clone()],
                                            },
                                        )));
                                        decl_emitted = true;
                                    }

                                    let prop_target = prefix_hidden(ONCE);
                                    // TODO: handle alias in fixed exports!
                                    let call = call_stmt(
                                        prop_target,
                                        name,
                                        Some(name.into()),
                                    );
                                    self.body.push(call);
                                } else if self
                                    .meta
                                    .live_export_map
                                    .contains_key(name)
                                {
                                    let prop_name = prefix_const(name);
                                    let prop_target = prefix_hidden(LIVE);
                                    if !decl_emitted {
                                        self.body.push(Stmt::Decl(Decl::Var(
                                            VarDecl {
                                                span: DUMMY_SP,
                                                kind: VarDeclKind::Let,
                                                declare: false,
                                                decls: vec![VarDeclarator {
                                                    span: DUMMY_SP,
                                                    name: Pat::Ident(
                                                        BindingIdent {
                                                            id: Ident {
                                                                span: DUMMY_SP,
                                                                sym: prop_name
                                                                    .clone(),
                                                                optional: false,
                                                            },
                                                            type_ann: None,
                                                        },
                                                    ),
                                                    init: decl.init.clone(),
                                                    definite: false,
                                                }],
                                            },
                                        )));
                                        decl_emitted = true;
                                    }

                                    // Hoisted references are an assignment
                                    if self.meta.hoisted_refs.contains(name) {
                                        self.body.push(
                                            Stmt::Expr(ExprStmt {
                                                span: DUMMY_SP,
                                                expr: Box::new(Expr::Assign(AssignExpr {
                                                    span: DUMMY_SP,
                                                    op: AssignOp::Assign,
                                                    left: PatOrExpr::Pat(Box::new(Pat::Ident(BindingIdent {
                                                        id: Ident {
                                                            span: DUMMY_SP,
                                                            optional: false,
                                                            sym: name.into(),
                                                        },
                                                        type_ann: None,
                                                    }))),
                                                    right: Box::new(Expr::Ident(Ident {
                                                        span: DUMMY_SP,
                                                        optional: false,
                                                        sym: prop_name.clone(),
                                                    }))
                                                })),
                                            })
                                        );
                                    // Otherwise a function call
                                    } else {
                                        let call = call_stmt(
                                            prop_target,
                                            name,
                                            Some(prop_name),
                                        );
                                        self.body.push(call);
                                    }
                                }
                            }
                        }
                    }
                    Decl::Fn(func) => {
                        // Rename the function so it matches the hoisted function statements
                        let target = prefix_const(func.ident.sym.as_ref());
                        let mut ident = func.ident.clone();
                        ident.sym = JsWord::from(target.clone());

                        // Output the function
                        self.body.push(Stmt::Expr(ExprStmt {
                            span: DUMMY_SP,
                            expr: Box::new(Expr::Fn(FnExpr {
                                ident: Some(ident),
                                function: func.function.clone(),
                            })),
                        }));
                    }
                    Decl::Class(class) => {
                        self.body.push(Stmt::Decl(Decl::Class(class.clone())));

                        // Set up the live export
                        let name = class.ident.sym.as_ref();
                        let prop_target = prefix_hidden(LIVE);
                        let call = call_stmt(
                            prop_target,
                            name,
                            Some(JsWord::from(name)),
                        );
                        self.body.push(call);
                    }
                    _ => {}
                },
                _ => {}
            },
            ModuleItem::Stmt(stmt) => self.visit_stmt(stmt, node),
        }
    }

    fn visit_stmt(&mut self, n: &Stmt, _: &dyn Node) {
        self.body.push(n.clone());
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
        self.hoist_exported_funcs(&mut block.stmts);
        self.hoist_exported_refs(&mut block.stmts);

        let mut visitor = Visitor {
            meta: self.meta,
            body: &mut block.stmts,
        };
        self.meta.module.visit_children_with(&mut visitor);

        block
    }

    fn hoist_exported_funcs(&self, stmts: &mut Vec<Stmt>) {
        for name in self.meta.hoisted_funcs.iter() {
            let target = prefix_const(name);

            // Use original `name` property for the function
            let define = define_property(target.as_ref(), NAME, name);
            stmts.push(define);

            // Set up the live export
            let prop_target = prefix_hidden(LIVE);
            let call = call_stmt(prop_target, name, Some(target));
            stmts.push(call);
        }
    }

    fn hoist_exported_refs(&self, stmts: &mut Vec<Stmt>) {
        for name in self.meta.hoisted_refs.iter() {
            // Set up the live export
            let prop_target = prefix_hidden(LIVE);
            let call = call_stmt(prop_target, name, None);
            stmts.push(call);
        }
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
        vec![ExprOrSpread {
            spread: None,
            expr: Box::new(Expr::Array(ArrayLit {
                span: DUMMY_SP,
                elems: {
                    let mut out = Vec::with_capacity(self.meta.imports.len());
                    let computed_aliases = self.meta.aliases();
                    for (key, props) in self.meta.imports.iter() {
                        let key: &str = &key[..];
                        let aliases = computed_aliases.get(key).unwrap();
                        let groups = self.group_duplicates(props, aliases);
                        out.push(Some(ExprOrSpread {
                            spread: None,
                            expr: Box::new(Expr::Array(ArrayLit {
                                span: DUMMY_SP,
                                elems: vec![
                                    Some(ExprOrSpread {
                                        spread: None,
                                        expr: Box::new(Expr::Lit(Lit::Str(
                                            Str {
                                                span: DUMMY_SP,
                                                kind: StrKind::Normal {
                                                    contains_quote: true,
                                                },
                                                value: key.into(),
                                                has_escape: false,
                                            },
                                        ))),
                                    }),
                                    Some(
                                        self.imports_map_constructor_args_map(
                                            groups
                                        ),
                                    ),
                                ],
                            })),
                        }));
                    }
                    out
                },
            })),
        }]
    }

    /// Group by raw name so that duplicates are rendered in the Map
    /// using the same key, eg: the wildcard `*` does not create multiple
    /// entries but the body instead assigns to multiple local variables.
    ///
    /// [
    ///     "./import-all-from-me.js",
    ///     new Map([["*",
    ///         [$h‍_a => (bar = $h‍_a),
    ///         $h‍_a => (baz = $h‍_a)]]])
    /// ]
    ///
    fn group_duplicates<'p, 's>(
        &self,
        props: &'p Vec<ImportName<'a>>,
        aliases: &'s Vec<&str>,
    ) -> IndexMap<&'p str, Vec<(&'p ImportName<'_>, &&'s str)>> {
        let mut out = IndexMap::new();
        for (prop, alias) in
            props.iter().zip(aliases.iter())
        {
            let name = prop.raw_name();
            let list = out.entry(name).or_insert(Vec::new());
            list.push((prop, alias));
        }
        out
    }

    /// The arguments for each nested map.
    fn imports_map_constructor_args_map(
        &self,
        groups: IndexMap<&str, Vec<(&ImportName<'_>, &&str)>>,
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
                            let mut out = Vec::with_capacity(groups.len());
                            for (key, list) in groups {
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
                                                        value: key.into(),
                                                        has_escape: false,
                                                    }),
                                                )),
                                            }),
                                            Some(ExprOrSpread {
                                                spread: None,
                                                expr: Box::new(Expr::Array(
                                                    ArrayLit {
                                                        span: DUMMY_SP,
                                                        elems: {
                                                            let mut items = Vec::with_capacity(list.len());
                                                            for (prop, alias) in list {
                                                                let name = prop.name;

                                                                let live = self
                                                                    .meta
                                                                    .live_export_map
                                                                    .contains_key(key)
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
                                                                                if v.0 == key {
                                                                                    return true;
                                                                                }
                                                                                false
                                                                            });

                                                                        live_alias.is_some()
                                                                    };

                                                                items.push(Some(
                                                                    self.imports_prop_func(name, alias, live)
                                                                ));
                                                            }
                                                            items
                                                        },
                                                    },
                                                )),
                                            }),
                                        ],
                                    })),
                                }));

                            }
                            out

                            //vec![]

                            /*
                            println!("Rendering with props {:#?}", props);
                            println!("Rendering with aliases {:#?}", aliases);

                            let mut out = Vec::with_capacity(props.len());
                            for (prop, alias) in
                                props.iter().zip(aliases.iter())
                            {
                                let name = prop.name;
                                let prop = prop.raw_name();

                                //println!("Key field is: {:#?}", prop);

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

                            }
                            out
                            */
                        },
                    })),
                }]),
                type_args: None,
            })),
        }
    }

    /// The import function which lazily assigns to the locally scoped variable.
    fn imports_prop_func(
        &self,
        name: &str,
        alias: &str,
        live: bool,
    ) -> ExprOrSpread {
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
                        value: name.into(),
                    }))),
                    computed: true,
                })),
            }
        } else {
            let arg = prefix_hidden("a");
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
