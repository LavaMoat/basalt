//! Generator the functor program from a static module record meta data.
use anyhow::Result;

use swc_atoms::JsWord;
use swc_ecma_ast::*;
use swc_common::DUMMY_SP;

use super::StaticModuleRecord;

const HIDDEN_PREFIX: &str = "$h\u{200d}_";
const HIDDEN_CONST_VAR_PREFIX:&str = "$c\u{200d}_";
const IMPORTS: &str = "imports";
const LIVE_VAR: &str = "liveVar";
const ONCE_VAR: &str = "onceVar";

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
                    body: BlockStmtOrExpr::BlockStmt(BlockStmt {
                        span: DUMMY_SP,
                        stmts: Vec::new(),
                    }),
                    is_async: false,
                    is_generator: false,
                    type_params: None,
                    return_type: None,
                }))
            })
            ),
        });

        script.body.push(stmt);

        Ok(script)
    }

    /// Build up the functor function parameters.
    fn params(&self) -> Vec<Pat> {
        let props = &[IMPORTS, LIVE_VAR, ONCE_VAR];
        vec![
            Pat::Object(ObjectPat {
                span: DUMMY_SP,
                props: {
                    let mut out = Vec::with_capacity(3);
                    for prop in props {
                        out.push(
                            ObjectPatProp::KeyValue(KeyValuePatProp {
                                key: PropName::Ident(Ident {
                                    span: DUMMY_SP,
                                    sym: (*prop).into(),
                                    optional: false,
                                }),
                                value: Box::new(Pat::Ident(BindingIdent{
                                    id: Ident {
                                        span: DUMMY_SP,
                                        sym: self.prefix_hidden(prop),
                                        optional: false,
                                    },
                                    type_ann: None,
                                }))
                            })
                        );
                    }
                    out
                },
                optional: false,
                type_ann: None,
            })
        ]
    }

    fn prefix_hidden(&self, word: &str) -> JsWord {
        format!("{}{}", HIDDEN_PREFIX, word).into()
    }
}
