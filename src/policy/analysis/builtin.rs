//! Analyze imports from builtin modules.
//!
//! Only finds builtin modules that are assigned;
//! side effect imports or calls to require will be ignored
//! under the assumption that built in modules would never
//! be designed for side effects.
//!
//! Due to [a bug](https://github.com/swc-project/swc/issues/1967) with
//! visiting all expressions when implementing this analysis currently this
//! is done as two passes; the first to gather import and require local symbols
//! and the second to detect usage and infer the access kind (RWX).
//!
//! Note that in the case of require calls that are shadowed
//! by an inner lexical scope then this analysis will result in false
//! positives.
//!
//! Note that `with` blocks are not evaluated relative to the target expression
//! so:
//!
//! ```javascript
//! with(process) {const foo = env.FOO};
//! ```
//!
//! Will only yield the entire `process` builtin and not the full path (`process.env.FOO`).
//!
use swc_atoms::JsWord;
use swc_ecma_ast::*;
use swc_ecma_visit::{Node, Visit, VisitWith};

use indexmap::IndexMap;

use crate::{
    access::{Access, AccessKind},
    policy::analysis::member_expr::{is_require_expr, member_expr_words},
};

use super::scope_builder::{Builtin, Local};

const FUNCTION_METHODS: [&str; 5] =
    ["call", "apply", "bind", "toSource", "toString"];

/// Visit a module and generate the set of access
/// to builtin packages.
#[derive(Default)]
#[deprecated(note = "Merged in with scope builder, use scope builder instead")]
pub struct BuiltinAnalysis;

impl BuiltinAnalysis {
    /// Analyze and compute the builtins for a module.
    pub fn analyze(
        &self,
        module: &Module,
        candidates: Vec<Builtin>,
    ) -> IndexMap<JsWord, Access> {
        let mut analyzer = BuiltinAnalyzer {
            candidates,
            access: Default::default(),
        };

        module.visit_children_with(&mut analyzer);

        self.compute(self.filter(analyzer.access))
    }

    /// Filter the list of access removing deep properties when an existing
    /// parent object already exists.
    ///
    /// The parent access is updated with any flags set on the child property access.
    fn filter(
        &self,
        map: IndexMap<Vec<JsWord>, Access>,
    ) -> IndexMap<Vec<JsWord>, Access> {
        let compare = map.clone();
        let mut updated: IndexMap<Vec<JsWord>, Access> = Default::default();
        let mut result: IndexMap<Vec<JsWord>, Access> = map
            .into_iter()
            .filter(|(k, access)| {
                for (key, parent_access) in compare.iter() {
                    if key.len() < k.len() {
                        if k.starts_with(&key) {
                            let mut new_access = parent_access.clone();
                            new_access.merge(&access);
                            updated.insert(key.clone(), new_access);
                            return false;
                        }
                    }
                }
                true
            })
            .collect();

        // Overwrite with updated access flags
        for (k, v) in updated {
            result.insert(k, v);
        }

        result
    }

    /// Compute the builtins.
    fn compute(
        &self,
        access: IndexMap<Vec<JsWord>, Access>,
    ) -> IndexMap<JsWord, Access> {
        let mut out: IndexMap<JsWord, Access> = Default::default();
        for (words, access) in access {
            let words: Vec<String> =
                words.into_iter().map(|w| w.as_ref().to_string()).collect();
            out.insert(JsWord::from(words.join(".")), access.clone());
        }
        out
    }
}

/// Analyze the imports and require calls to built in modules.
struct BuiltinAnalyzer {
    candidates: Vec<Builtin>,
    access: IndexMap<Vec<JsWord>, Access>,
}

impl BuiltinAnalyzer {
    /// Determine if a word matches a previously located builtin module local
    /// symbol. For member expressions pass the first word in the expression.
    fn is_builtin_match(&self, sym: &JsWord) -> Option<(&Local, JsWord)> {
        for builtin in self.candidates.iter() {
            let mut matched = builtin.locals.iter().find(|local| {
                let word = match local {
                    Local::Default(word) => word,
                    Local::Named(word) => word,
                    Local::Alias(word, _) => word,
                };
                word == sym
            });
            if let Some(local) = matched.take() {
                return Some((local, builtin.source.clone()));
            }
        }
        None
    }

    fn insert_access(&mut self, words_key: Vec<JsWord>, kind: &AccessKind) {
        let entry = self.access.entry(words_key).or_insert(Default::default());
        match kind {
            AccessKind::Read => {
                entry.read = true;
            }
            AccessKind::Write => {
                entry.write = true;
            }
            AccessKind::Execute => {
                entry.execute = true;
            }
        }
    }

    fn access_visit_expr(&mut self, n: &Expr, kind: &AccessKind) {
        match n {
            Expr::Ident(n) => {
                if let Some((local, source)) = self.is_builtin_match(&n.sym) {
                    let words_key = if let Local::Alias(_, alias) = local {
                        vec![source, alias.clone()]
                    } else {
                        if source == n.sym {
                            vec![source]
                        } else {
                            vec![source, n.sym.clone()]
                        }
                    };

                    self.insert_access(words_key, kind);
                }
            }
            Expr::New(n) => {
                self.access_visit_expr(&*n.callee, &AccessKind::Read);
            }
            Expr::Fn(n) => {
                self.access_visit_fn(&n.function);
            }
            Expr::Arrow(n) => {
                for pat in &n.params {
                    self.access_visit_pat(pat);
                }
                match &n.body {
                    BlockStmtOrExpr::Expr(n) => {
                        self.access_visit_expr(n, kind);
                    }
                    BlockStmtOrExpr::BlockStmt(n) => {
                        for stmt in &n.stmts {
                            self.access_visit_stmt(stmt);
                        }
                    }
                }
            }
            Expr::Member(member) => {
                if is_require_expr(n).is_none() {
                    // TODO: ensure the first word is Expr::Ident!
                    let members = member_expr_words(member);
                    if let Some(word) = members.get(0) {
                        if let Some((local, source)) =
                            self.is_builtin_match(word)
                        {
                            let mut words_key: Vec<JsWord> =
                                members.into_iter().cloned().collect();
                            if let Some(word) = words_key.get(0) {
                                if word != &source {
                                    if let Local::Default(_) = local {
                                        words_key.remove(0);
                                    }
                                    words_key.insert(0, source.clone());
                                }
                            }

                            if let Local::Alias(_word, alias) = local {
                                words_key = vec![source, alias.clone()];
                            }

                            // Strip function methods like `call`, `apply` and `bind` etc.
                            if let AccessKind::Execute = kind {
                                if let Some(last) = words_key.last() {
                                    if FUNCTION_METHODS.contains(&last.as_ref())
                                    {
                                        words_key.pop();
                                    }
                                }
                            }

                            self.insert_access(words_key, kind);
                        }
                    }
                }
            }
            // Update is a write access
            Expr::Update(n) => {
                self.access_visit_expr(&*n.arg, &AccessKind::Write);
            }
            // Execute access is a function call
            Expr::Call(call) => {
                if is_require_expr(n).is_none() {
                    for arg in &call.args {
                        self.access_visit_expr(&*arg.expr, &AccessKind::Read);
                    }
                    match &call.callee {
                        ExprOrSuper::Expr(n) => {
                            self.access_visit_expr(n, &AccessKind::Execute);
                        }
                        _ => {}
                    }
                }
            }
            // Write access on left-hand side of an assignment
            Expr::Assign(n) => {
                match &n.left {
                    PatOrExpr::Pat(n) => match &**n {
                        Pat::Ident(n) => {
                            if let Some((local, source)) =
                                self.is_builtin_match(&n.id.sym)
                            {
                                let words_key = match local {
                                    Local::Named(word) => {
                                        vec![source, word.clone()]
                                    }
                                    Local::Default(_word) => vec![source],
                                    Local::Alias(_word, alias) => {
                                        vec![source, alias.clone()]
                                    }
                                };

                                self.insert_access(
                                    words_key,
                                    &AccessKind::Write,
                                );
                            }
                        }
                        Pat::Expr(n) => {
                            self.access_visit_expr(n, &AccessKind::Write);
                        }
                        _ => {}
                    },
                    _ => {}
                }
                self.access_visit_expr(&n.right, kind);
            }
            Expr::Paren(n) => {
                self.access_visit_expr(&*n.expr, kind);
            }
            Expr::OptChain(n) => {
                self.access_visit_expr(&*n.expr, kind);
            }
            Expr::Unary(n) => {
                self.access_visit_expr(&n.arg, kind);
            }
            Expr::Await(n) => {
                self.access_visit_expr(&n.arg, kind);
            }
            Expr::Yield(n) => {
                if let Some(arg) = &n.arg {
                    self.access_visit_expr(arg, kind);
                }
            }
            Expr::Bin(n) => {
                self.access_visit_expr(&*n.left, kind);
                self.access_visit_expr(&*n.right, kind);
            }
            Expr::Cond(n) => {
                self.access_visit_expr(&*n.test, kind);
                self.access_visit_expr(&*n.cons, kind);
                self.access_visit_expr(&*n.alt, kind);
            }
            Expr::Tpl(n) => {
                for expr in n.exprs.iter() {
                    self.access_visit_expr(&*expr, kind);
                }
            }
            Expr::TaggedTpl(n) => {
                self.access_visit_expr(&*n.tag, kind);
                for expr in n.tpl.exprs.iter() {
                    self.access_visit_expr(&*expr, kind);
                }
            }
            _ => {}
        }
    }

    fn access_visit_stmt(&mut self, n: &Stmt) {
        match n {
            Stmt::Return(n) => {
                if let Some(arg) = &n.arg {
                    self.access_visit_expr(&*arg, &AccessKind::Read);
                }
            }
            Stmt::Decl(n) => match &n {
                Decl::Fn(n) => {
                    self.access_visit_fn(&n.function);
                }
                Decl::Class(n) => {
                    if let Some(super_class) = &n.class.super_class {
                        self.access_visit_expr(super_class, &AccessKind::Read);
                    }

                    for member in &n.class.body {
                        match member {
                            ClassMember::Constructor(n) => {
                                for param in &n.params {
                                    if let ParamOrTsParamProp::Param(param) =
                                        param
                                    {
                                        self.access_visit_pat(&param.pat);
                                    }
                                }

                                if let Some(body) = &n.body {
                                    for n in &body.stmts {
                                        self.access_visit_stmt(n);
                                    }
                                }
                            }
                            ClassMember::Method(n) => {
                                self.access_visit_fn(&n.function);
                            }
                            ClassMember::PrivateMethod(n) => {
                                self.access_visit_fn(&n.function);
                            }
                            ClassMember::ClassProp(n) => {
                                if let Some(n) = &n.value {
                                    self.access_visit_expr(
                                        n,
                                        &AccessKind::Read,
                                    );
                                }
                            }
                            ClassMember::PrivateProp(n) => {
                                if let Some(n) = &n.value {
                                    self.access_visit_expr(
                                        n,
                                        &AccessKind::Read,
                                    );
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Decl::Var(n) => {
                    for decl in &n.decls {
                        if let Some(init) = &decl.init {
                            self.access_visit_expr(init, &AccessKind::Read);
                        }
                    }
                }
                _ => {}
            },
            Stmt::Block(n) => {
                for n in &n.stmts {
                    self.access_visit_stmt(n);
                }
            }
            Stmt::Expr(n) => {
                self.access_visit_expr(&n.expr, &AccessKind::Read);
            }
            Stmt::With(n) => {
                self.access_visit_expr(&*n.obj, &AccessKind::Read);
                self.access_visit_stmt(&*n.body);
            }
            Stmt::Labeled(n) => {
                self.access_visit_stmt(&*n.body);
            }
            Stmt::If(n) => {
                self.access_visit_expr(&*n.test, &AccessKind::Read);
                self.access_visit_stmt(&*n.cons);
                if let Some(alt) = &n.alt {
                    self.access_visit_stmt(&*alt);
                }
            }
            Stmt::Switch(n) => {
                self.access_visit_expr(&*n.discriminant, &AccessKind::Read);
                for case in &n.cases {
                    if let Some(test) = &case.test {
                        self.access_visit_expr(&*test, &AccessKind::Read);
                    }
                    for stmt in &case.cons {
                        self.access_visit_stmt(stmt);
                    }
                }
            }
            Stmt::Throw(n) => {
                self.access_visit_expr(&*n.arg, &AccessKind::Read);
            }
            Stmt::Try(n) => {
                for n in &n.block.stmts {
                    self.access_visit_stmt(n);
                }

                if let Some(handler) = &n.handler {
                    for n in &handler.body.stmts {
                        self.access_visit_stmt(n);
                    }
                }

                if let Some(finalizer) = &n.finalizer {
                    for n in &finalizer.stmts {
                        self.access_visit_stmt(n);
                    }
                }
            }
            Stmt::While(n) => {
                self.access_visit_expr(&*n.test, &AccessKind::Read);
                self.access_visit_stmt(&*n.body);
            }
            Stmt::DoWhile(n) => {
                self.access_visit_expr(&*n.test, &AccessKind::Read);
                self.access_visit_stmt(&*n.body);
            }
            Stmt::For(n) => {
                if let Some(init) = &n.init {
                    self.access_visit_var_decl_or_expr(init);
                }
                if let Some(test) = &n.test {
                    self.access_visit_expr(&*test, &AccessKind::Read);
                }
                if let Some(update) = &n.update {
                    self.access_visit_expr(&*update, &AccessKind::Read);
                }
                self.access_visit_stmt(&*n.body);
            }
            Stmt::ForIn(n) => {
                self.access_visit_var_decl_or_pat(&n.left);
                self.access_visit_expr(&*n.right, &AccessKind::Read);
                self.access_visit_stmt(&*n.body);
            }
            Stmt::ForOf(n) => {
                self.access_visit_var_decl_or_pat(&n.left);
                self.access_visit_expr(&*n.right, &AccessKind::Read);
                self.access_visit_stmt(&*n.body);
            }
            _ => {}
        }
    }

    fn access_visit_fn(&mut self, n: &Function) {
        for param in &n.params {
            self.access_visit_pat(&param.pat);
        }
        if let Some(body) = &n.body {
            for n in &body.stmts {
                self.access_visit_stmt(n);
            }
        }
    }

    fn access_visit_pat(&mut self, n: &Pat) {
        // FIXME: Handle other variants
        match n {
            Pat::Assign(n) => {
                // Right hand side of assignment
                self.access_visit_expr(&*n.right, &AccessKind::Read);
            }
            // Needed for for..of and for..in loops
            Pat::Expr(n) => self.access_visit_expr(n, &AccessKind::Read),
            _ => {}
        }
    }

    fn access_visit_var_decl(&mut self, n: &VarDecl) {
        for decl in &n.decls {
            if let Some(init) = &decl.init {
                self.access_visit_expr(init, &AccessKind::Read);
            }
        }
    }

    fn access_visit_var_decl_or_pat(&mut self, n: &VarDeclOrPat) {
        match n {
            VarDeclOrPat::VarDecl(n) => {
                self.access_visit_var_decl(n);
            }
            VarDeclOrPat::Pat(n) => {
                self.access_visit_pat(n);
            }
        }
    }

    fn access_visit_var_decl_or_expr(&mut self, n: &VarDeclOrExpr) {
        match n {
            VarDeclOrExpr::VarDecl(n) => {
                self.access_visit_var_decl(n);
            }
            VarDeclOrExpr::Expr(n) => {
                self.access_visit_expr(n, &AccessKind::Read);
            }
        }
    }
}

impl Visit for BuiltinAnalyzer {
    fn visit_stmt(&mut self, n: &Stmt, _: &dyn Node) {
        self.access_visit_stmt(n);
    }
}
