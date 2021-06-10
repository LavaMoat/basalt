//! Generator the functor program from a static module record meta data.
use anyhow::Result;

use swc_ecma_ast::*;
use swc_common::DUMMY_SP;

use super::StaticModuleRecord;

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
                    params: Vec::new(),
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
}
