//! Analyze the lexical scopes for a module and generate a tree
//! containing the local symbols and a list of identities which are
//! symbol references that maybe global variables.

use swc_atoms::JsWord;
use swc_common::DUMMY_SP;
use swc_ecma_ast::*;
use swc_ecma_visit::{Node, Visit};

use indexmap::IndexSet;
use serde::{Serialize, Serializer};

use crate::helpers::{pattern_words, var_symbol_words};

fn visit_member<'a>(
    n: &'a MemberExpr,
    words: &mut Vec<&'a JsWord>,
) {
    let mut is_visitable = !n.computed;
    match &n.obj {
        ExprOrSuper::Expr(expr) => {
            match &**expr {
                Expr::Ident(_) => visit_member_expr(expr, words),
                Expr::Call(n) => {
                    match &n.callee {
                        ExprOrSuper::Expr(expr) => {
                            match &**expr {
                                Expr::Ident(id) => {
                                    words.push(&id.sym);
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                    is_visitable = false;
                }
                // NOTE: Don't handle other expressions such as `Call`
                // NOTE: otherwise we generate for `fetch().then()`
                _ => {
                    is_visitable = false;
                },
            }
        }
        _ => {}
    }

    // Recurse or visit if the property is not computed.
    //
    // If the property were computed we would just return the parent object.
    //
    // console['log']('foo');
    //
    // Would return `console`.
    if is_visitable {
        match &*n.prop {
            Expr::Member(n) => visit_member(n, words),
            _ => visit_member_expr(&*n.prop, words),
        }
    }
}

fn visit_member_expr<'a>(
    n: &'a Expr,
    words: &mut Vec<&'a JsWord>,
) {
    match n {
        Expr::Ident(id) => {
            words.push(&id.sym);
        }
        _ => {}
    }
}

/// Reference to a symbol.
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum Ref {
    /// Simple symbol reference.
    Word(JsWord),
    /// Member expression.
    Member(MemberExpr),
}

/*
impl PartialEq for Ref {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Ref::Word(lhs), Ref::Word(rhs)) => lhs == rhs,
            (Ref::Member(lhs), Ref::Member(rhs)) => {
                lhs.obj == rhs.obj
            }
            _ => false,
        }
    }
}
*/

impl Ref {
    /// Get a list of all the words for a reference.
    pub fn words(&self) -> Vec<&JsWord> {
        match self {
            Ref::Word(word) => vec![word],
            Ref::Member(n) => {
                let mut words = Vec::new();
                visit_member(n, &mut words);
                words
            }
        }
    }

    /// Get a dot-delimited path for the words in a reference.
    pub fn path(&self) -> String {
        let words = self
            .words()
            .iter()
            .map(|w| w.as_ref().to_string())
            .collect::<Vec<_>>();
        words.join(".")
    }
}

impl From<JsWord> for Ref {
    fn from(word: JsWord) -> Self {
        Ref::Word(word)
    }
}

impl From<&JsWord> for Ref {
    fn from(word: &JsWord) -> Self {
        Ref::Word(word.clone())
    }
}

impl Serialize for Ref {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Ref::Word(word) => serializer.serialize_str(word.as_ref()),
            Ref::Member(_) => {
                let dot_path = self.path();
                serializer.serialize_str(&dot_path)
            }
        }
    }
}

/// Lexical scope.
#[derive(Debug)]
pub struct Scope {
    /// Scopes contained by this scope.
    scopes: Vec<Scope>,
    /// Identifiers local to this scope.
    locals: IndexSet<Ref>,
    /// Identifiers that are references.
    ///
    /// These could be local or global symbols and we need
    /// to walk all parent scopes to detect if a symbol should
    /// be considered global.
    idents: IndexSet<Ref>,
}

impl Scope {
    fn new(locals: Option<IndexSet<Ref>>) -> Self {
        Self {
            scopes: Default::default(),
            locals: locals.unwrap_or(Default::default()),
            idents: Default::default(),
        }
    }
}

/// Analyze the scopes for a module.
#[derive(Debug)]
pub struct ScopeAnalysis {
    root: Scope,
}

impl ScopeAnalysis {
    /// Create a scope analysis.
    pub fn new() -> Self {
        Self {
            root: Scope::new(None),
        }
    }

    /// Compute the global variables.
    pub fn globals(&self) -> IndexSet<Ref> {
        let mut global_symbols: IndexSet<Ref> = Default::default();
        self.compute_globals(&self.root, &mut global_symbols, &mut vec![]);
        global_symbols
    }

    fn compute_globals<'a>(
        &self,
        scope: &'a Scope,
        global_symbols: &mut IndexSet<Ref>,
        locals_stack: &mut Vec<&'a IndexSet<Ref>>,
    ) {
        locals_stack.push(&scope.locals);

        let mut combined_locals: IndexSet<Ref> = Default::default();
        for locals in locals_stack.iter() {
            combined_locals = combined_locals.union(locals).cloned().collect();
        }

        let mut diff: IndexSet<Ref> =
            scope.idents.difference(&combined_locals).cloned().collect();
        for sym in diff.drain(..) {
            global_symbols.insert(sym.into());
        }

        for scope in scope.scopes.iter() {
            self.compute_globals(scope, global_symbols, locals_stack);
        }

        locals_stack.pop();
    }
}

impl Visit for ScopeAnalysis {
    fn visit_module_item(&mut self, n: &ModuleItem, _: &dyn Node) {
        let scope = &mut self.root;
        match n {
            ModuleItem::ModuleDecl(decl) => match decl {
                ModuleDecl::Import(import) => {
                    for spec in import.specifiers.iter() {
                        let id = match spec {
                            ImportSpecifier::Named(n) => &n.local.sym,
                            ImportSpecifier::Default(n) => &n.local.sym,
                            ImportSpecifier::Namespace(n) => &n.local.sym,
                        };
                        scope.locals.insert(id.into());
                    }
                }
                _ => {}
            },
            ModuleItem::Stmt(stmt) => visit_stmt(stmt, scope, None),
        }
    }
}

fn visit_stmt(n: &Stmt, scope: &mut Scope, locals: Option<IndexSet<Ref>>) {
    match n {
        Stmt::Decl(decl) => {
            match decl {
                Decl::Fn(n) => {
                    scope.locals.insert((&n.ident.sym).into());
                    visit_function(&n.function, scope, Default::default());
                }
                Decl::Class(n) => {
                    scope.locals.insert((&n.ident.sym).into());
                    visit_class(&n.class, scope, None);
                }
                Decl::Var(n) => {
                    visit_var_decl(n, scope);
                }
                _ => {}
            };
        }
        Stmt::With(n) => {
            let mut next_scope = Scope::new(None);
            visit_stmt(&*n.body, &mut next_scope, None);
            scope.scopes.push(next_scope);
        }
        Stmt::While(n) => {
            let mut next_scope = Scope::new(None);
            visit_stmt(&*n.body, &mut next_scope, None);
            scope.scopes.push(next_scope);
        }
        Stmt::DoWhile(n) => {
            let mut next_scope = Scope::new(None);
            visit_stmt(&*n.body, &mut next_scope, None);
            scope.scopes.push(next_scope);
        }
        Stmt::For(n) => {
            let mut next_scope = Scope::new(None);
            if let Some(init) = &n.init {
                match init {
                    VarDeclOrExpr::Expr(n) => {
                        visit_expr(n, &mut next_scope);
                    }
                    VarDeclOrExpr::VarDecl(n) => {
                        visit_var_decl(n, &mut next_scope);
                    }
                }
            }

            if let Some(test) = &n.test {
                visit_expr(test, &mut next_scope);
            }
            if let Some(update) = &n.update {
                visit_expr(update, &mut next_scope);
            }

            visit_stmt(&*n.body, &mut next_scope, None);
            scope.scopes.push(next_scope);
        }
        Stmt::ForIn(n) => {
            let mut next_scope = Scope::new(None);
            visit_stmt(&*n.body, &mut next_scope, None);
            scope.scopes.push(next_scope);
        }
        Stmt::ForOf(n) => {
            let mut next_scope = Scope::new(None);
            visit_stmt(&*n.body, &mut next_scope, None);
            scope.scopes.push(next_scope);
        }
        Stmt::Labeled(n) => {
            let mut next_scope = Scope::new(None);
            visit_stmt(&*n.body, &mut next_scope, None);
            scope.scopes.push(next_scope);
        }
        Stmt::If(n) => {
            let mut next_scope = Scope::new(None);
            visit_stmt(&*n.cons, &mut next_scope, None);
            scope.scopes.push(next_scope);

            if let Some(ref alt) = n.alt {
                let mut next_scope = Scope::new(None);
                visit_stmt(&*alt, &mut next_scope, None);
                scope.scopes.push(next_scope);
            }
        }
        Stmt::Try(n) => {
            let block_stmt = Stmt::Block(n.block.clone());
            let mut next_scope = Scope::new(None);
            visit_stmt(&block_stmt, &mut next_scope, None);
            scope.scopes.push(next_scope);

            if let Some(ref catch_clause) = n.handler {
                let block_stmt = Stmt::Block(catch_clause.body.clone());
                let mut next_scope = Scope::new(None);
                visit_stmt(&block_stmt, &mut next_scope, None);
                scope.scopes.push(next_scope);
            }

            if let Some(ref finalizer) = n.finalizer {
                let block_stmt = Stmt::Block(finalizer.clone());
                let mut next_scope = Scope::new(None);
                visit_stmt(&block_stmt, &mut next_scope, None);
                scope.scopes.push(next_scope);
            }
        }
        Stmt::Switch(n) => {
            for case in n.cases.iter() {
                for stmt in case.cons.iter() {
                    let mut next_scope = Scope::new(None);
                    visit_stmt(stmt, &mut next_scope, None);
                    scope.scopes.push(next_scope);
                }
            }
        }
        Stmt::Block(n) => {
            let mut next_scope = Scope::new(locals);
            for stmt in n.stmts.iter() {
                visit_stmt(stmt, &mut next_scope, None);
            }
            scope.scopes.push(next_scope);
        }
        Stmt::Return(n) => {
            if let Some(arg) = &n.arg {
                visit_expr(arg, scope);
            }
        }
        Stmt::Throw(n) => {
            visit_expr(&*n.arg, scope);
        }
        // Find symbol references which is the list of candidates
        // that may be global (or local) variable references.
        Stmt::Expr(n) => visit_expr(&*n.expr, scope),
        _ => {}
    }
}

fn visit_expr(n: &Expr, scope: &mut Scope) {
    match n {
        Expr::Ident(id) => {
            scope.idents.insert((&id.sym).into());
        }
        Expr::PrivateName(n) => {
            scope.idents.insert(private_name_prefix(&n.id.sym));
        }
        Expr::Tpl(n) => {
            for expr in n.exprs.iter() {
                visit_expr(&*expr, scope);
            }
        }
        Expr::TaggedTpl(n) => {
            visit_expr(&*n.tag, scope);
            for expr in n.tpl.exprs.iter() {
                visit_expr(&*expr, scope);
            }
        }
        Expr::Seq(n) => {
            for expr in n.exprs.iter() {
                visit_expr(&*expr, scope);
            }
        }
        Expr::Array(n) => {
            for elem in n.elems.iter() {
                if let Some(elem) = elem {
                    visit_expr(&elem.expr, scope);
                }
            }
        }
        Expr::Object(n) => {
            for prop in n.props.iter() {
                match prop {
                    PropOrSpread::Spread(n) => {
                        visit_expr(&*n.expr, scope);
                    }
                    PropOrSpread::Prop(n) => match &**n {
                        Prop::Shorthand(id) => {
                            scope.idents.insert((&id.sym).into());
                        }
                        Prop::KeyValue(n) => {
                            visit_expr(&*n.value, scope);
                        }
                        _ => {}
                    },
                }
            }
        }
        Expr::Paren(n) => {
            visit_expr(&n.expr, scope);
        }
        Expr::Yield(n) => {
            if let Some(ref arg) = n.arg {
                visit_expr(arg, scope);
            }
        }
        Expr::Cond(n) => {
            visit_expr(&*n.test, scope);
            visit_expr(&*n.cons, scope);
            visit_expr(&*n.alt, scope);
        }
        Expr::Await(n) => {
            visit_expr(&n.arg, scope);
        }
        Expr::Arrow(n) => {
            let mut func_param_names: IndexSet<Ref> = Default::default();

            // Capture arrow function parameters as locals
            for pat in n.params.iter() {
                let mut names = Vec::new();
                pattern_words(pat, &mut names);
                let param_names: IndexSet<_> =
                    names.into_iter().map(|n| n.into()).collect();
                func_param_names =
                    func_param_names.union(&param_names).cloned().collect();
            }

            match &n.body {
                BlockStmtOrExpr::BlockStmt(block) => {
                    let block_stmt = Stmt::Block(block.clone());
                    visit_stmt(&block_stmt, scope, Some(func_param_names));
                }
                BlockStmtOrExpr::Expr(expr) => {
                    let expr_stmt = Stmt::Expr(ExprStmt {
                        span: DUMMY_SP,
                        expr: expr.clone(),
                    });
                    visit_stmt(&expr_stmt, scope, Some(func_param_names));
                }
            }
        }
        Expr::Call(n) => match &n.callee {
            ExprOrSuper::Expr(expr) => {
                visit_expr(expr, scope);
            }
            _ => {}
        },
        Expr::Update(n) => {
            visit_expr(&n.arg, scope);
        }
        Expr::Unary(n) => {
            visit_expr(&n.arg, scope);
        }
        Expr::Assign(assign) => {
            match &assign.left {
                PatOrExpr::Expr(expr) => {
                    visit_expr(expr, scope);
                }
                PatOrExpr::Pat(pat) => match &**pat {
                    Pat::Ident(ident) => {
                        scope.idents.insert((&ident.id.sym).into());
                    }
                    _ => {}
                },
            }
            visit_expr(&*assign.right, scope);
        }
        Expr::OptChain(_) => {
            todo!("Handle optional chaining operator: ?.");
        }
        Expr::Member(n) => {
            match &n.obj {
                ExprOrSuper::Expr(expr) => {
                    match **expr {
                        Expr::This(_) => {},
                        _ => {
                            scope.idents.insert(Ref::Member(n.clone()));
                        },
                    }
                }
                _ => {}
            }
        }
        Expr::New(n) => {
            visit_expr(&*n.callee, scope);
        }
        Expr::Fn(n) => {
            // Named function expressions only expose the name
            // to the inner scope like class expressions.
            let locals = n
                .ident
                .as_ref()
                .map(|id| {
                    let mut set = IndexSet::new();
                    set.insert((&id.sym).into());
                    set
                })
                .unwrap_or_default();
            visit_function(&n.function, scope, locals);
        }
        Expr::Class(n) => {
            // Class expressions with a named identifer like:
            //
            // const Foo = class FooNamed {}
            //
            // Only expose the class name (FooNamed) to the inner
            // scope so we pass in the locals.
            let locals = n.ident.as_ref().map(|id| {
                let mut set = IndexSet::new();
                set.insert((&id.sym).into());
                set
            });
            visit_class(&n.class, scope, locals);
        }
        _ => {}
    }
}

fn visit_class(n: &Class, scope: &mut Scope, locals: Option<IndexSet<Ref>>) {
    let mut next_scope = Scope::new(locals);

    // In case the super class reference is a global
    if let Some(ref super_class) = n.super_class {
        visit_expr(&*super_class, scope);
    }

    for member in n.body.iter() {
        match member {
            ClassMember::Constructor(n) => {
                if let Some(body) = &n.body {
                    let block_stmt = Stmt::Block(body.clone());
                    visit_stmt(&block_stmt, &mut next_scope, None);
                }
            }
            ClassMember::Method(n) => {
                if !n.is_static {
                    match &n.key {
                        PropName::Ident(id) => {
                            scope.locals.insert((&id.sym).into());
                        }
                        _ => {}
                    }
                    visit_function(
                        &n.function,
                        &mut next_scope,
                        Default::default(),
                    );
                }
            }
            ClassMember::PrivateMethod(n) => {
                if !n.is_static {
                    scope.locals.insert((&n.key.id.sym).into());
                    visit_function(
                        &n.function,
                        &mut next_scope,
                        Default::default(),
                    );
                }
            }
            ClassMember::ClassProp(n) => {
                if !n.is_static {
                    // TODO: Should we handle other types of expressions here?
                    match &*n.key {
                        Expr::Ident(ident) => {
                            scope.locals.insert((&ident.sym).into());
                        }
                        _ => {}
                    }
                    if let Some(value) = &n.value {
                        visit_expr(value, &mut next_scope);
                    }
                }
            }
            ClassMember::PrivateProp(n) => {
                if !n.is_static {
                    scope.locals.insert(private_name_prefix(&n.key.id.sym));
                    if let Some(value) = &n.value {
                        visit_expr(value, &mut next_scope);
                    }
                }
            }
            _ => {}
        }
    }

    scope.scopes.push(next_scope);
}

fn visit_function(n: &Function, scope: &mut Scope, mut locals: IndexSet<Ref>) {
    // Capture function parameters as locals
    for param in n.params.iter() {
        let mut names = Vec::new();
        pattern_words(&param.pat, &mut names);
        let param_names: IndexSet<_> =
            names.into_iter().map(|n| n.into()).collect();
        locals = locals.union(&param_names).cloned().collect();
    }

    if let Some(ref body) = n.body {
        let block_stmt = Stmt::Block(body.clone());
        visit_stmt(&block_stmt, scope, Some(locals));
    }
}

fn visit_var_decl(n: &VarDecl, scope: &mut Scope) {
    let word_list = var_symbol_words(n);
    for (decl, words) in word_list.iter() {
        for word in words {
            scope.locals.insert((*word).into());
        }

        // Recurse on variable declarations with initializers
        if let Some(ref init) = decl.init {
            visit_expr(init, scope);
        }
    }
}

// The JsWord for PrivateName is stripped of the # symbol
// but that would mean that they would incorrectly shadow
// so we restore it.
fn private_name_prefix(word: &JsWord) -> Ref {
    Ref::from(JsWord::from(format!("#{}", word.as_ref())))
}
