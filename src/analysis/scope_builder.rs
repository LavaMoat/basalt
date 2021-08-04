//! Analyze the lexical scopes for a module and generate a tree
//! containing the local symbols and a list of identities which are
//! symbol references.
//!

use swc_atoms::JsWord;
use swc_ecma_ast::*;
use swc_ecma_visit::{Node, Visit, VisitAll, VisitAllWith, VisitWith};

use indexmap::IndexSet;

use super::member_expr::walk;
use crate::helpers::{pattern_words, var_symbol_words};

const GLOBAL_THIS: &str = "globalThis";

/// Represents a symbol word or a member expression path.
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum WordOrPath {
    /// Symbol word.
    Word(JsWord),
    /// Member expression path.
    Path(JsWord, Vec<JsWord>),
}

impl WordOrPath {
    /// Convert into a dot delimited path.
    pub fn into_path(&self) -> JsWord {
        match self {
            WordOrPath::Word(word) => word.clone(),
            WordOrPath::Path(word, parts) => {
                let mut words: Vec<&JsWord> = parts.iter().collect();
                words.insert(0, word);
                let words: Vec<String> =
                    words.iter().map(|w| w.as_ref().to_string()).collect();
                JsWord::from(words.join("."))
            }
        }
    }
}

impl Into<JsWord> for &WordOrPath {
    fn into(self) -> JsWord {
        // This implementation only returns the first word
        // so that detection of member expressions when computing
        // globals is correct when comparing against locals which
        // should always be a single word.
        match self {
            WordOrPath::Word(word) => word.clone(),
            WordOrPath::Path(word, _) => word.clone(),
        }
    }
}

/// Lexical scope.
#[derive(Debug)]
pub struct Scope {
    /// Scopes contained by this scope.
    pub scopes: Vec<Scope>,
    /// Identifiers local to this scope.
    pub locals: IndexSet<JsWord>,
    /// Identifiers that are references.
    ///
    /// These could be local or global symbols and we need
    /// to combine all parent scopes to detect if a symbol should
    /// be considered global.
    pub idents: IndexSet<WordOrPath>,
}

impl Scope {
    /// Create a scope.
    pub fn new(locals: Option<IndexSet<JsWord>>) -> Self {
        Self {
            scopes: Default::default(),
            locals: locals.unwrap_or(Default::default()),
            idents: Default::default(),
        }
    }
}

/// Scope builder creates a tree of scopes.
#[derive(Debug)]
pub struct ScopeBuilder;

impl ScopeBuilder {

    /// Visit a statement.
    pub fn _visit_stmt(
        &self,
        n: &Stmt,
        scope: &mut Scope,
        locals: Option<IndexSet<JsWord>>,
    ) {
        match n {
            Stmt::Decl(decl) => {
                match decl {
                    Decl::Fn(n) => {
                        scope.locals.insert(n.ident.sym.clone());
                        self._visit_function(
                            &n.function,
                            scope,
                            Default::default(),
                        );
                    }
                    Decl::Class(n) => {
                        scope.locals.insert(n.ident.sym.clone());
                        self._visit_class(&n.class, scope, None);
                    }
                    Decl::Var(n) => {
                        self._visit_var_decl(n, scope);
                    }
                    _ => {}
                };
            }
            Stmt::With(n) => {
                let mut next_scope = Scope::new(None);
                self._visit_stmt(&*n.body, &mut next_scope, None);
                scope.scopes.push(next_scope);
            }
            Stmt::While(n) => {
                let mut next_scope = Scope::new(None);
                self._visit_stmt(&*n.body, &mut next_scope, None);
                scope.scopes.push(next_scope);
            }
            Stmt::DoWhile(n) => {
                let mut next_scope = Scope::new(None);
                self._visit_stmt(&*n.body, &mut next_scope, None);
                scope.scopes.push(next_scope);
            }
            Stmt::For(n) => {
                let mut next_scope = Scope::new(None);
                if let Some(init) = &n.init {
                    match init {
                        VarDeclOrExpr::Expr(n) => {
                            self._visit_expr(n, &mut next_scope);
                        }
                        VarDeclOrExpr::VarDecl(n) => {
                            self._visit_var_decl(n, &mut next_scope);
                        }
                    }
                }

                if let Some(test) = &n.test {
                    self._visit_expr(test, &mut next_scope);
                }
                if let Some(update) = &n.update {
                    self._visit_expr(update, &mut next_scope);
                }

                self._visit_stmt(&*n.body, &mut next_scope, None);
                scope.scopes.push(next_scope);
            }
            Stmt::ForIn(n) => {
                let mut next_scope = Scope::new(None);

                match &n.left {
                    VarDeclOrPat::VarDecl(n) => {
                        self._visit_var_decl(n, &mut next_scope);
                    }
                    VarDeclOrPat::Pat(pat) => match pat {
                        Pat::Expr(n) => {
                            self._visit_expr(n, &mut next_scope);
                        }
                        _ => {
                            let mut names = Vec::new();
                            pattern_words(pat, &mut names);
                            for sym in names {
                                self.insert_ident(sym.clone(), scope, None);
                            }
                        }
                    },
                }

                self._visit_expr(&*n.right, &mut next_scope);

                self._visit_stmt(&*n.body, &mut next_scope, None);
                scope.scopes.push(next_scope);
            }
            Stmt::ForOf(n) => {
                let mut next_scope = Scope::new(None);

                match &n.left {
                    VarDeclOrPat::VarDecl(n) => {
                        self._visit_var_decl(n, &mut next_scope);
                    }
                    VarDeclOrPat::Pat(pat) => match pat {
                        Pat::Expr(n) => {
                            self._visit_expr(n, &mut next_scope);
                        }
                        _ => {
                            let mut names = Vec::new();
                            pattern_words(pat, &mut names);
                            for sym in names {
                                self.insert_ident(sym.clone(), scope, None);
                            }
                        }
                    },
                }

                self._visit_expr(&*n.right, &mut next_scope);

                self._visit_stmt(&*n.body, &mut next_scope, None);
                scope.scopes.push(next_scope);
            }
            Stmt::Labeled(n) => {
                let mut next_scope = Scope::new(None);
                self._visit_stmt(&*n.body, &mut next_scope, None);
                scope.scopes.push(next_scope);
            }
            Stmt::If(n) => {
                let mut next_scope = Scope::new(None);
                self._visit_expr(&*n.test, &mut next_scope);
                self._visit_stmt(&*n.cons, &mut next_scope, None);
                scope.scopes.push(next_scope);

                if let Some(ref alt) = n.alt {
                    let mut next_scope = Scope::new(None);
                    self._visit_stmt(&*alt, &mut next_scope, None);
                    scope.scopes.push(next_scope);
                }
            }
            Stmt::Try(n) => {
                let block_stmt = Stmt::Block(n.block.clone());
                let mut next_scope = Scope::new(None);
                self._visit_stmt(&block_stmt, &mut next_scope, None);
                scope.scopes.push(next_scope);

                if let Some(ref catch_clause) = n.handler {
                    let locals = if let Some(pat) = &catch_clause.param {
                        let mut names = Vec::new();
                        pattern_words(pat, &mut names);
                        let locals: IndexSet<_> =
                            names.into_iter().map(|n| n.clone()).collect();
                        Some(locals)
                    } else {
                        None
                    };
                    let block_stmt = Stmt::Block(catch_clause.body.clone());
                    let mut next_scope = Scope::new(locals);
                    self._visit_stmt(&block_stmt, &mut next_scope, None);
                    scope.scopes.push(next_scope);
                }

                if let Some(ref finalizer) = n.finalizer {
                    let block_stmt = Stmt::Block(finalizer.clone());
                    let mut next_scope = Scope::new(None);
                    self._visit_stmt(&block_stmt, &mut next_scope, None);
                    scope.scopes.push(next_scope);
                }
            }
            Stmt::Switch(n) => {
                for case in n.cases.iter() {
                    for stmt in case.cons.iter() {
                        let mut next_scope = Scope::new(None);
                        self._visit_stmt(stmt, &mut next_scope, None);
                        scope.scopes.push(next_scope);
                    }
                }
            }
            Stmt::Block(n) => {
                let mut next_scope = Scope::new(locals);
                for stmt in n.stmts.iter() {
                    self._visit_stmt(stmt, &mut next_scope, None);
                }
                scope.scopes.push(next_scope);
            }
            Stmt::Return(n) => {
                if let Some(arg) = &n.arg {
                    self._visit_expr(arg, scope);
                }
            }
            Stmt::Throw(n) => {
                self._visit_expr(&*n.arg, scope);
            }
            Stmt::Expr(n) => self._visit_expr(&*n.expr, scope),
            _ => {}
        }
    }

    fn _visit_expr(&self, n: &Expr, scope: &mut Scope) {
        match n {
            Expr::Ident(id) => {
                self.insert_ident(id.sym.clone(), scope, None);
            }
            Expr::PrivateName(n) => {
                self.insert_ident(private_name_prefix(&n.id.sym), scope, None);
            }
            Expr::Bin(n) => {
                self._visit_expr(&*n.left, scope);
                self._visit_expr(&*n.right, scope);
            }
            Expr::Tpl(n) => {
                for expr in n.exprs.iter() {
                    self._visit_expr(&*expr, scope);
                }
            }
            Expr::TaggedTpl(n) => {
                self._visit_expr(&*n.tag, scope);
                for expr in n.tpl.exprs.iter() {
                    self._visit_expr(&*expr, scope);
                }
            }
            Expr::Seq(n) => {
                for expr in n.exprs.iter() {
                    self._visit_expr(&*expr, scope);
                }
            }
            Expr::Array(n) => {
                for elem in n.elems.iter() {
                    if let Some(elem) = elem {
                        self._visit_expr(&elem.expr, scope);
                    }
                }
            }
            Expr::Object(n) => {
                for prop in n.props.iter() {
                    match prop {
                        PropOrSpread::Spread(n) => {
                            self._visit_expr(&*n.expr, scope);
                        }
                        PropOrSpread::Prop(n) => match &**n {
                            Prop::Shorthand(id) => {
                                self.insert_ident(id.sym.clone(), scope, None);
                            }
                            Prop::KeyValue(n) => {
                                self._visit_expr(&*n.value, scope);
                            }
                            _ => {}
                        },
                    }
                }
            }
            Expr::Paren(n) => {
                self._visit_expr(&n.expr, scope);
            }
            Expr::Yield(n) => {
                if let Some(ref arg) = n.arg {
                    self._visit_expr(arg, scope);
                }
            }
            Expr::Cond(n) => {
                self._visit_expr(&*n.test, scope);
                self._visit_expr(&*n.cons, scope);
                self._visit_expr(&*n.alt, scope);
            }
            Expr::Await(n) => {
                self._visit_expr(&n.arg, scope);
            }
            Expr::Arrow(n) => {
                let mut func_param_names: IndexSet<JsWord> = Default::default();

                // Capture arrow function parameters as locals
                for pat in n.params.iter() {
                    let mut names = Vec::new();
                    pattern_words(pat, &mut names);
                    let param_names: IndexSet<_> =
                        names.into_iter().map(|n| n.clone()).collect();
                    func_param_names =
                        func_param_names.union(&param_names).cloned().collect();
                }

                match &n.body {
                    BlockStmtOrExpr::BlockStmt(block) => {
                        let block_stmt = Stmt::Block(block.clone());
                        self._visit_stmt(
                            &block_stmt,
                            scope,
                            Some(func_param_names),
                        );
                    }
                    BlockStmtOrExpr::Expr(expr) => {
                        scope.locals = scope
                            .locals
                            .union(&func_param_names)
                            .cloned()
                            .collect();
                        self._visit_expr(expr, scope);
                    }
                }
            }
            Expr::Call(n) => {
                match &n.callee {
                    ExprOrSuper::Expr(expr) => {
                        self._visit_expr(expr, scope);
                    }
                    _ => {}
                }
                for arg in &n.args {
                    self._visit_expr(&*arg.expr, scope);
                }
            }
            Expr::Update(n) => {
                self._visit_expr(&n.arg, scope);
            }
            Expr::Unary(n) => {
                self._visit_expr(&n.arg, scope);
            }
            Expr::Assign(assign) => {
                match &assign.left {
                    PatOrExpr::Expr(expr) => {
                        self._visit_expr(expr, scope);
                    }
                    PatOrExpr::Pat(pat) => match &**pat {
                        Pat::Ident(ident) => {
                            self.insert_ident(
                                ident.id.sym.clone(),
                                scope,
                                None,
                            );
                        }
                        _ => {}
                    },
                }
                self._visit_expr(&*assign.right, scope);
            }
            Expr::OptChain(n) => {
                self._visit_expr(&n.expr, scope);
            }
            Expr::Member(n) => {
                let members = self.compute_member(n, scope);
                for (word, parts) in members {
                    self.insert_ident(word, scope, Some(parts));
                }
            }
            Expr::New(n) => {
                self._visit_expr(&*n.callee, scope);
            }
            Expr::Fn(n) => {
                // Named function expressions only expose the name
                // to the inner scope like class expressions.
                let locals = n
                    .ident
                    .as_ref()
                    .map(|id| {
                        let mut set = IndexSet::new();
                        set.insert(id.sym.clone());
                        set
                    })
                    .unwrap_or_default();
                self._visit_function(&n.function, scope, locals);
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
                    set.insert(id.sym.clone());
                    set
                });
                self._visit_class(&n.class, scope, locals);
            }
            _ => {}
        }
    }

    fn _visit_class(
        &self,
        n: &Class,
        scope: &mut Scope,
        locals: Option<IndexSet<JsWord>>,
    ) {
        let mut next_scope = Scope::new(locals);

        // In case the super class reference is a global
        if let Some(ref super_class) = n.super_class {
            self._visit_expr(&*super_class, scope);
        }

        for member in n.body.iter() {
            match member {
                ClassMember::Constructor(n) => {
                    for param in &n.params {
                        if let ParamOrTsParamProp::Param(param) = param {
                            self._visit_param(param, scope);
                        }
                    }

                    if let Some(body) = &n.body {
                        let block_stmt = Stmt::Block(body.clone());
                        self._visit_stmt(&block_stmt, &mut next_scope, None);
                    }
                }
                ClassMember::Method(n) => {
                    if !n.is_static {
                        match &n.key {
                            PropName::Ident(id) => {
                                scope.locals.insert(id.sym.clone());
                            }
                            _ => {}
                        }
                        self._visit_function(
                            &n.function,
                            &mut next_scope,
                            Default::default(),
                        );
                    }
                }
                ClassMember::PrivateMethod(n) => {
                    if !n.is_static {
                        scope.locals.insert(n.key.id.sym.clone());
                        self._visit_function(
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
                                scope.locals.insert(ident.sym.clone());
                            }
                            _ => {}
                        }
                        if let Some(value) = &n.value {
                            self._visit_expr(value, &mut next_scope);
                        }
                    }
                }
                ClassMember::PrivateProp(n) => {
                    if !n.is_static {
                        scope.locals.insert(private_name_prefix(&n.key.id.sym));
                        if let Some(value) = &n.value {
                            self._visit_expr(value, &mut next_scope);
                        }
                    }
                }
                _ => {}
            }
        }

        scope.scopes.push(next_scope);
    }

    fn _visit_function(
        &self,
        n: &Function,
        scope: &mut Scope,
        locals: IndexSet<JsWord>,
    ) {
        // Capture function parameters as locals
        for param in n.params.iter() {
            self._visit_param(param, scope);
        }

        if let Some(ref body) = n.body {
            let block_stmt = Stmt::Block(body.clone());
            self._visit_stmt(&block_stmt, scope, Some(locals));
        }
    }

    fn _visit_param(&self, n: &Param, scope: &mut Scope) {
        let mut names = Vec::new();
        pattern_words(&n.pat, &mut names);
        let param_names: IndexSet<_> =
            names.into_iter().map(|n| n.clone()).collect();

        // NOTE: Must re-assign locals before checking assign patterns so that
        // NOTE: later pattern assignments can reference previously declared
        // NOTE: parameters, eg:
        // NOTE:
        // NOTE: function toComputedKey(node, key = node.key || node.property)
        scope.locals = scope.locals.union(&param_names).cloned().collect();

        // Handle arguments with default values
        //
        // eg: `function foo(win = window) {}`
        if let Pat::Assign(n) = &n.pat {
            self._visit_expr(&*n.right, scope);
        }
    }

    fn _visit_var_decl(&self, n: &VarDecl, scope: &mut Scope) {
        let word_list = var_symbol_words(n);
        for (decl, words) in word_list.iter() {
            for word in words {
                scope.locals.insert((*word).clone());
            }

            // Recurse on variable declarations with initializers
            if let Some(ref init) = decl.init {
                self._visit_expr(init, scope);
            }
        }
    }

    fn compute_member(
        &self,
        n: &MemberExpr,
        scope: &mut Scope,
    ) -> Vec<(JsWord, Vec<JsWord>)> {
        let mut members = Vec::new();

        let mut expressions = Vec::new();
        walk(n, &mut expressions);

        if let Some(first) = expressions.get(0) {
            match first {
                Expr::This(_) => {
                    return members;
                }
                Expr::Ident(id) => {
                    if id.sym.as_ref() == GLOBAL_THIS {
                        expressions.remove(0);
                    }
                }
                _ => {
                    let mut visit_paren = VisitMemberParen {
                        members: Vec::new(),
                        idents: Vec::new(),
                    };
                    n.visit_all_children_with(&mut visit_paren);
                    for member in visit_paren.members.iter() {
                        let mut result = self.compute_member(member, scope);
                        members.append(&mut result);
                    }

                    for ident in visit_paren.idents.into_iter() {
                        self.insert_ident(ident.sym, scope, None);
                    }
                }
            }
        }

        if let Some(member_expr) = self.compute_member_words(&mut expressions) {
            members.push(member_expr);
        }

        members
    }

    fn compute_member_words(
        &self,
        expressions: &Vec<&Expr>,
    ) -> Option<(JsWord, Vec<JsWord>)> {
        let mut words: Vec<JsWord> = Vec::new();
        for expr in expressions.iter() {
            match expr {
                Expr::Ident(id) => {
                    words.push(id.sym.clone());
                }
                Expr::Call(call) => {
                    if let ExprOrSuper::Expr(expr) = &call.callee {
                        match &**expr {
                            Expr::Ident(id) => {
                                words.push(id.sym.clone());
                            }
                            _ => {}
                        }
                    }
                    break;
                }
                _ => break,
            }
        }

        if words.is_empty() {
            None
        } else {
            Some((words.remove(0), words))
        }
    }

    fn insert_ident(
        &self,
        sym: JsWord,
        scope: &mut Scope,
        path: Option<Vec<JsWord>>,
    ) {
        let word_or_path = if let Some(path) = path {
            WordOrPath::Path(sym, path)
        } else {
            WordOrPath::Word(sym)
        };
        scope.idents.insert(word_or_path);
    }
}

// Find nested parentheses in a member expression and then
// search for nested member expressions within the parentheses.
struct VisitMemberParen {
    members: Vec<MemberExpr>,
    idents: Vec<Ident>,
}

impl VisitAll for VisitMemberParen {
    fn visit_paren_expr(&mut self, n: &ParenExpr, _: &dyn Node) {
        let mut visit_members = VisitNestedMembers {
            members: Vec::new(),
            idents: Vec::new(),
        };
        n.visit_children_with(&mut visit_members);
        for member in visit_members.members.drain(..) {
            self.members.push(member);
        }

        for id in visit_members.idents.drain(..) {
            self.idents.push(id);
        }
    }
}

struct VisitNestedMembers {
    members: Vec<MemberExpr>,
    idents: Vec<Ident>,
}

impl Visit for VisitNestedMembers {
    fn visit_member_expr(&mut self, n: &MemberExpr, _: &dyn Node) {
        self.members.push(n.clone());
    }

    fn visit_ident(&mut self, n: &Ident, _: &dyn Node) {
        self.idents.push(n.clone());
    }
}

// The JsWord for PrivateName is stripped of the # symbol
// but that would mean that they would incorrectly shadow
// so we restore it.
fn private_name_prefix(word: &JsWord) -> JsWord {
    JsWord::from(format!("#{}", word.as_ref()))
}
