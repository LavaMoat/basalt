//! Analyze the lexical scopes for a module and generate a tree
//! containing the local symbols and a list of identities which are
//! symbol references.
//!

use std::cell::RefCell;
use std::rc::Rc;

use swc_atoms::JsWord;
use swc_ecma_ast::*;
use swc_ecma_visit::{Node, Visit, VisitAll, VisitAllWith, VisitWith};

use indexmap::IndexSet;

use crate::{
    helpers::{pattern_words, var_symbol_words},
    module::dependencies::is_builtin_module,
    policy::analysis::member_expr::{is_require_expr, member_expr_words, walk},
};

const GLOBAL_THIS: &str = "globalThis";

const FUNCTION_METHODS: [&str; 5] =
    ["call", "apply", "bind", "toSource", "toString"];

/// Reference to a built in module.
///
/// May be from an import specifier, call to `require()` or a dynamic `import()`.
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct Builtin {
    pub(crate) source: JsWord,
    pub(crate) locals: Vec<Local>,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub(crate) enum Local {
    Default(JsWord),
    // Named locals will need to be converted to fully qualified
    // module paths, eg: `readSync` would become the canonical `fs.readSync`
    Named(JsWord),

    // The local symbol is the first word and the alias is the second.
    Alias(JsWord, JsWord),
}

/// Enumeration of function variants in the AST.
///
/// Used for unified handling of functions regardless of type.
enum Func<'a> {
    Fn(&'a Function),
    Constructor(&'a Constructor),
    Arrow(&'a ArrowExpr),
}

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

impl Into<Vec<JsWord>> for &WordOrPath {
    fn into(self) -> Vec<JsWord> {
        match self {
            WordOrPath::Word(word) => vec![word.clone()],
            WordOrPath::Path(word, parts) => {
                let mut out = vec![word.clone()];
                for word in parts {
                    out.push(word.clone());
                }
                out
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
    /// Hoisted variable declarations.
    pub hoisted_vars: Rc<RefCell<IndexSet<JsWord>>>,
}

impl Scope {
    /// Create a scope.
    pub fn new(
        locals: Option<IndexSet<JsWord>>,
        hoisted_vars: Rc<RefCell<IndexSet<JsWord>>>,
    ) -> Self {
        Self {
            scopes: Default::default(),
            locals: locals.unwrap_or(Default::default()),
            idents: Default::default(),
            hoisted_vars,
        }
    }

    /// Create a scope with locals and owned hoisted variables.
    pub fn locals(locals: Option<IndexSet<JsWord>>) -> Self {
        Self {
            scopes: Default::default(),
            locals: locals.unwrap_or(Default::default()),
            idents: Default::default(),
            hoisted_vars: Rc::new(RefCell::new(Default::default())),
        }
    }

    fn from_parent(parent: &mut Scope) -> Self {
        Scope::new(None, Rc::clone(&parent.hoisted_vars))
    }
}

/// Scope builder creates a tree of scopes.
#[derive(Debug, Default)]
pub struct ScopeBuilder {
    /// Builtin module detection candidates.
    pub candidates: Vec<Builtin>,
    /// List of symbols that reference a builtin candidate.
    pub builtins: IndexSet<Vec<JsWord>>,
}

impl ScopeBuilder {
    /// Add a static import declaration.
    pub fn add_static_import(&mut self, n: &ImportDecl) {
        if is_builtin_module(n.src.value.as_ref()) {
            let mut builtin = Builtin {
                source: n.src.value.clone(),
                locals: Default::default(),
            };
            for spec in n.specifiers.iter() {
                let local = match spec {
                    ImportSpecifier::Default(n) => {
                        Local::Default(n.local.sym.clone())
                    }
                    ImportSpecifier::Named(n) => {
                        Local::Named(n.local.sym.clone())
                    }
                    ImportSpecifier::Namespace(n) => {
                        Local::Default(n.local.sym.clone())
                    }
                };
                if !builtin.locals.contains(&local) {
                    builtin.locals.push(local);
                }
            }
            self.candidates.push(builtin);
        }
    }

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

    #[inline(always)]
    fn insert_builtin(&mut self, words_key: Vec<JsWord>) {
        self.builtins.insert(words_key);
    }

    /// Visit a statement.
    pub fn visit_stmt(
        &mut self,
        n: &Stmt,
        scope: &mut Scope,
        locals: Option<IndexSet<JsWord>>,
    ) {
        match n {
            Stmt::Decl(decl) => {
                match decl {
                    Decl::Fn(n) => {
                        scope.locals.insert(n.ident.sym.clone());
                        self.visit_function(Func::Fn(&n.function), scope, None);
                    }
                    Decl::Class(n) => {
                        scope.locals.insert(n.ident.sym.clone());
                        self.visit_class(&n.class, scope, None);
                    }
                    Decl::Var(n) => {
                        self.visit_var_decl(n, scope);
                    }
                    _ => {}
                };
            }
            Stmt::With(n) => {
                let mut next_scope = Scope::from_parent(scope);
                self.visit_stmt(&*n.body, &mut next_scope, None);
                scope.scopes.push(next_scope);
            }
            Stmt::While(n) => {
                self.visit_expr(&*n.test, scope);
                let mut next_scope = Scope::from_parent(scope);
                self.visit_stmt(&*n.body, &mut next_scope, None);
                scope.scopes.push(next_scope);
            }
            Stmt::DoWhile(n) => {
                self.visit_expr(&*n.test, scope);
                let mut next_scope = Scope::from_parent(scope);
                self.visit_stmt(&*n.body, &mut next_scope, None);
                scope.scopes.push(next_scope);
            }
            Stmt::For(n) => {
                let mut next_scope = Scope::from_parent(scope);
                if let Some(init) = &n.init {
                    match init {
                        VarDeclOrExpr::Expr(n) => {
                            self.visit_expr(n, &mut next_scope);
                        }
                        VarDeclOrExpr::VarDecl(n) => {
                            self.visit_var_decl(n, &mut next_scope);
                        }
                    }
                }

                if let Some(test) = &n.test {
                    self.visit_expr(test, &mut next_scope);
                }
                if let Some(update) = &n.update {
                    self.visit_expr(update, &mut next_scope);
                }

                self.visit_stmt(&*n.body, &mut next_scope, None);
                scope.scopes.push(next_scope);
            }
            Stmt::ForIn(n) => {
                let mut next_scope = Scope::from_parent(scope);
                match &n.left {
                    VarDeclOrPat::VarDecl(n) => {
                        self.visit_var_decl(n, &mut next_scope);
                    }
                    VarDeclOrPat::Pat(pat) => match pat {
                        Pat::Expr(n) => {
                            self.visit_expr(n, &mut next_scope);
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

                self.visit_expr(&*n.right, &mut next_scope);

                self.visit_stmt(&*n.body, &mut next_scope, None);
                scope.scopes.push(next_scope);
            }
            Stmt::ForOf(n) => {
                let mut next_scope = Scope::from_parent(scope);
                match &n.left {
                    VarDeclOrPat::VarDecl(n) => {
                        self.visit_var_decl(n, &mut next_scope);
                    }
                    VarDeclOrPat::Pat(pat) => match pat {
                        Pat::Expr(n) => {
                            self.visit_expr(n, &mut next_scope);
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

                self.visit_expr(&*n.right, &mut next_scope);

                self.visit_stmt(&*n.body, &mut next_scope, None);
                scope.scopes.push(next_scope);
            }
            Stmt::Labeled(n) => {
                let mut next_scope = Scope::from_parent(scope);
                self.visit_stmt(&*n.body, &mut next_scope, None);
                scope.scopes.push(next_scope);
            }
            Stmt::If(n) => {
                let mut next_scope = Scope::from_parent(scope);
                self.visit_expr(&*n.test, &mut next_scope);
                self.visit_stmt(&*n.cons, &mut next_scope, None);
                scope.scopes.push(next_scope);

                if let Some(ref alt) = n.alt {
                    let mut next_scope = Scope::from_parent(scope);
                    self.visit_stmt(&*alt, &mut next_scope, None);
                    scope.scopes.push(next_scope);
                }
            }
            Stmt::Try(n) => {
                let mut next_scope = Scope::from_parent(scope);
                self.visit_block_stmt(&n.block, &mut next_scope);
                scope.scopes.push(next_scope);

                if let Some(catch_clause) = &n.handler {
                    let locals = if let Some(pat) = &catch_clause.param {
                        let mut names = Vec::new();
                        pattern_words(pat, &mut names);
                        let locals: IndexSet<_> =
                            names.into_iter().map(|n| n.clone()).collect();
                        Some(locals)
                    } else {
                        None
                    };

                    let mut next_scope =
                        Scope::new(locals, Rc::clone(&scope.hoisted_vars));
                    self.visit_block_stmt(&catch_clause.body, &mut next_scope);
                    scope.scopes.push(next_scope);
                }

                if let Some(finalizer) = &n.finalizer {
                    let mut next_scope = Scope::from_parent(scope);
                    self.visit_block_stmt(finalizer, &mut next_scope);
                    scope.scopes.push(next_scope);
                }
            }
            Stmt::Switch(n) => {
                for case in n.cases.iter() {
                    for stmt in case.cons.iter() {
                        let mut next_scope = Scope::from_parent(scope);
                        self.visit_stmt(stmt, &mut next_scope, None);
                        scope.scopes.push(next_scope);
                    }
                }
            }
            Stmt::Block(n) => {
                let mut next_scope =
                    Scope::new(locals, Rc::clone(&scope.hoisted_vars));
                for stmt in n.stmts.iter() {
                    self.visit_stmt(stmt, &mut next_scope, None);
                }
                scope.scopes.push(next_scope);
            }
            Stmt::Return(n) => {
                if let Some(arg) = &n.arg {
                    self.visit_expr(arg, scope);
                }
            }
            Stmt::Throw(n) => {
                self.visit_expr(&*n.arg, scope);
            }
            Stmt::Expr(n) => self.visit_expr(&*n.expr, scope),
            _ => {}
        }
    }

    fn visit_expr(&mut self, n: &Expr, scope: &mut Scope) {
        match n {
            Expr::Ident(n) => {
                self.insert_ident(n.sym.clone(), scope, None);
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

                    self.insert_builtin(words_key);
                }
            }
            Expr::PrivateName(n) => {
                self.insert_ident(private_name_prefix(&n.id.sym), scope, None);
            }
            Expr::Bin(n) => {
                self.visit_expr(&*n.left, scope);
                self.visit_expr(&*n.right, scope);
            }
            Expr::Tpl(n) => {
                for expr in n.exprs.iter() {
                    self.visit_expr(&*expr, scope);
                }
            }
            Expr::TaggedTpl(n) => {
                self.visit_expr(&*n.tag, scope);
                for expr in n.tpl.exprs.iter() {
                    self.visit_expr(&*expr, scope);
                }
            }
            Expr::Seq(n) => {
                for expr in n.exprs.iter() {
                    self.visit_expr(&*expr, scope);
                }
            }
            Expr::Array(n) => {
                for elem in n.elems.iter() {
                    if let Some(elem) = elem {
                        self.visit_expr(&elem.expr, scope);
                    }
                }
            }
            Expr::Object(n) => {
                for prop in n.props.iter() {
                    match prop {
                        PropOrSpread::Spread(n) => {
                            self.visit_expr(&*n.expr, scope);
                        }
                        PropOrSpread::Prop(n) => match &**n {
                            Prop::Shorthand(id) => {
                                self.insert_ident(id.sym.clone(), scope, None);
                            }
                            Prop::KeyValue(n) => {
                                self.visit_expr(&*n.value, scope);
                            }
                            _ => {}
                        },
                    }
                }
            }
            Expr::Paren(n) => {
                self.visit_expr(&n.expr, scope);
            }
            Expr::Yield(n) => {
                if let Some(ref arg) = n.arg {
                    self.visit_expr(arg, scope);
                }
            }
            Expr::Cond(n) => {
                self.visit_expr(&*n.test, scope);
                self.visit_expr(&*n.cons, scope);
                self.visit_expr(&*n.alt, scope);
            }
            Expr::Await(n) => {
                self.visit_expr(&n.arg, scope);
            }
            Expr::Arrow(n) => {
                self.visit_function(Func::Arrow(n), scope, None);
            }
            Expr::Call(n) => {
                match &n.callee {
                    ExprOrSuper::Expr(expr) => {
                        self.visit_expr(expr, scope);
                    }
                    _ => {}
                }
                for arg in &n.args {
                    self.visit_expr(&*arg.expr, scope);
                }
            }
            Expr::Update(n) => {
                self.visit_expr(&n.arg, scope);
            }
            Expr::Unary(n) => {
                self.visit_expr(&n.arg, scope);
            }
            Expr::Assign(assign) => {
                match &assign.left {
                    PatOrExpr::Expr(expr) => {
                        self.visit_expr(expr, scope);
                    }
                    PatOrExpr::Pat(pat) => match &**pat {
                        Pat::Ident(ident) => {
                            if let Some((local, source)) =
                                self.is_builtin_match(&ident.id.sym)
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

                                self.insert_builtin(words_key);
                            }

                            self.insert_ident(
                                ident.id.sym.clone(),
                                scope,
                                None,
                            );
                        }
                        Pat::Expr(expr) => self.visit_expr(expr, scope),
                        _ => {}
                    },
                }
                self.visit_expr(&*assign.right, scope);
            }
            Expr::OptChain(n) => {
                self.visit_expr(&n.expr, scope);
            }
            Expr::Member(member) => {
                let members = self.compute_member(member, scope);
                for (word, parts) in members {
                    self.insert_ident(word, scope, Some(parts));
                }

                // Builtin handling
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

                            // FIXME: only apply this logic for function calls (execute access)

                            // Strip function methods like `call`, `apply` and `bind` etc.
                            if let Some(last) = words_key.last() {
                                if FUNCTION_METHODS.contains(&last.as_ref()) {
                                    words_key.pop();
                                }
                            }

                            self.insert_builtin(words_key);
                        }
                    }
                }
            }
            Expr::New(n) => {
                self.visit_expr(&*n.callee, scope);
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
                self.visit_function(Func::Fn(&n.function), scope, Some(locals));
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
                self.visit_class(&n.class, scope, locals);
            }
            _ => {}
        }
    }

    fn visit_class(
        &mut self,
        n: &Class,
        scope: &mut Scope,
        locals: Option<IndexSet<JsWord>>,
    ) {
        let mut next_scope = Scope::new(locals, Rc::clone(&scope.hoisted_vars));

        // In case the super class reference is a global
        if let Some(ref super_class) = n.super_class {
            self.visit_expr(&*super_class, scope);
        }

        for member in n.body.iter() {
            match member {
                ClassMember::Constructor(n) => {
                    self.visit_function(
                        Func::Constructor(n),
                        &mut next_scope,
                        None,
                    );
                }
                ClassMember::Method(n) => {
                    if !n.is_static {
                        self.visit_function(
                            Func::Fn(&n.function),
                            &mut next_scope,
                            None,
                        );
                    }
                }
                ClassMember::PrivateMethod(n) => {
                    if !n.is_static {
                        self.visit_function(
                            Func::Fn(&n.function),
                            &mut next_scope,
                            None,
                        );
                    }
                }
                ClassMember::ClassProp(n) => {
                    if !n.is_static {
                        if let Some(value) = &n.value {
                            self.visit_expr(value, &mut next_scope);
                        }
                    }
                }
                ClassMember::PrivateProp(n) => {
                    if !n.is_static {
                        if let Some(value) = &n.value {
                            self.visit_expr(value, &mut next_scope);
                        }
                    }
                }
                _ => {}
            }
        }

        scope.scopes.push(next_scope);
    }

    fn visit_function(
        &mut self,
        n: Func,
        scope: &mut Scope,
        locals: Option<IndexSet<JsWord>>,
    ) {
        let mut next_scope = Scope::new(locals, Rc::clone(&scope.hoisted_vars));

        // Gether function parameters
        let params = match n {
            Func::Fn(n) => n.params.iter().map(|n| &n.pat).collect(),
            Func::Arrow(n) => n.params.iter().collect(),
            Func::Constructor(n) => {
                let mut params = Vec::new();
                for param in &n.params {
                    if let ParamOrTsParamProp::Param(param) = param {
                        params.push(&param.pat);
                    }
                }
                params
            }
        };

        // Capture function parameters as locals
        for pat in params {
            self.visit_param_pat(pat, &mut next_scope);
        }

        let body = match n {
            Func::Fn(n) => n.body.as_ref(),
            Func::Constructor(n) => n.body.as_ref(),
            Func::Arrow(n) => match &n.body {
                BlockStmtOrExpr::BlockStmt(block) => Some(block),
                BlockStmtOrExpr::Expr(expr) => {
                    self.visit_expr(expr, &mut next_scope);
                    None
                }
            },
        };

        if let Some(body) = &body {
            self.visit_block_stmt(body, &mut next_scope);
        }

        scope.scopes.push(next_scope);
    }

    fn visit_block_stmt(&mut self, n: &BlockStmt, scope: &mut Scope) {
        for stmt in &n.stmts {
            self.visit_stmt(stmt, scope, None);
        }
    }

    fn visit_param_pat(&mut self, n: &Pat, scope: &mut Scope) {
        let mut names = Vec::new();
        pattern_words(n, &mut names);
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
        if let Pat::Assign(n) = &n {
            self.visit_expr(&*n.right, scope);
        }
    }

    fn visit_var_decl(&mut self, n: &VarDecl, scope: &mut Scope) {
        let word_list = var_symbol_words(n);
        for (decl, words) in word_list.iter() {
            for word in words {
                match n.kind {
                    VarDeclKind::Var => {
                        let mut hoisted = scope.hoisted_vars.borrow_mut();
                        hoisted.insert((*word).clone());
                    }
                    _ => {
                        scope.locals.insert((*word).clone());
                    }
                }
            }

            // Recurse on variable declarations with initializers
            if let Some(init) = &decl.init {
                self.visit_expr(init, scope);
            }

            self.visit_var_declarator(decl, scope);
        }
    }

    fn visit_var_declarator(&mut self, n: &VarDeclarator, _scope: &mut Scope) {
        if let Some(init) = &n.init {
            if let Some((name, member_name)) = is_require_expr(init) {
                if is_builtin_module(name.as_ref()) {
                    let mut builtin = Builtin {
                        source: name.clone(),
                        locals: Default::default(),
                    };
                    builtin.locals = match &n.name {
                        // Looks like a default require statement
                        // but may have dot access so we test
                        // for a member name.
                        Pat::Ident(ident) => {
                            if let Some(member_name) = member_name {
                                vec![Local::Alias(
                                    ident.id.sym.clone(),
                                    member_name.clone(),
                                )]
                            } else {
                                vec![Local::Default(ident.id.sym.clone())]
                            }
                        }
                        // Handle object destructuring on LHS of require()
                        _ => {
                            let mut names = Vec::new();
                            pattern_words(&n.name, &mut names);
                            names
                                .into_iter()
                                .cloned()
                                .map(|sym| Local::Named(sym))
                                .collect()
                        }
                    };
                    self.candidates.push(builtin);
                }
            }
        }
    }

    fn compute_member(
        &mut self,
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
        &mut self,
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

    #[inline(always)]
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

impl VisitNestedMembers {
    fn _visit_expr(&mut self, n: &Expr) {
        // FIXME: all the paths for nested member expressions should be declared!
        match n {
            Expr::Ident(n) => {
                self.idents.push(n.clone());
            }
            Expr::Bin(n) => {
                self._visit_expr(&*n.left);
                self._visit_expr(&*n.right);
            }
            Expr::Member(n) => {
                self.members.push(n.clone());
            }
            _ => {}
        }
    }
}

impl Visit for VisitNestedMembers {
    fn visit_expr(&mut self, n: &Expr, _: &dyn Node) {
        self._visit_expr(n);
    }
}

// The JsWord for PrivateName is stripped of the # symbol
// but that would mean that they would incorrectly shadow
// so we restore it.
fn private_name_prefix(word: &JsWord) -> JsWord {
    JsWord::from(format!("#{}", word.as_ref()))
}
