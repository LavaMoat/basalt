//! Analyze the lexical scopes for a module and generate a tree
//! containing the local symbols and a list of identities which are
//! symbol references that maybe global variables.

use swc_atoms::JsWord;
use swc_common::DUMMY_SP;
use swc_ecma_ast::*;
use swc_ecma_visit::{Node, Visit};

use indexmap::IndexSet;

use crate::helpers::{pattern_words, var_symbol_words};

// SEE: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects

static REQUIRE: &str = "require";
static MODULE: &str = "module";
static EXPORTS: &str = "exports";
// TODO
static GLOBAL_THIS: &str = "globalThis";

static KEYWORDS: [&'static str; 3] = ["undefined", "NaN", "Infinity"];

static INTRINSICS: [&'static str; 51] = [
    // Fundamental objects
    "Object",
    "Function",
    "Boolean",
    "Symbol",
    // Error objects
    "Error",
    "AggregateError",
    "EvalError",
    "InternalError",
    "RangeError",
    "ReferenceError",
    "SyntaxError",
    "TypeError",
    "URIError",
    // Numbers and dates
    "Number",
    "BigInt",
    "Math",
    "Date",
    // Text processing
    "String",
    "RegExp",
    // Indexed collections
    "Array",
    "Int8Array",
    "Uint8Array",
    "Uint8ClampedArray",
    "Int16Array",
    "Uint16Array",
    "Int32Array",
    "Uint32Array",
    "Float32Array",
    "Float64Array",
    "BigInt64Array",
    "BigUint64Array",
    // Keyed collections
    "Map",
    "Set",
    "WeakMap",
    "WeakSet",
    // Structured data
    "ArrayBuffer",
    "SharedArrayBuffer",
    "Atomics",
    "DataView",
    "JSON",
    // Control abstraction objects
    "Promise",
    "Generator",
    "GeneratorFunction",
    "AsyncFunction",
    "AsyncGenerator",
    "AsyncGeneratorFunction",
    // Reflection
    "Reflect",
    "Proxy",
    // Internationalization
    "Intl",
    // Webassembly
    "WebAssembly",
    // Other
    "arguments",
];

/// Processing options for the global analysis.
#[derive(Debug, Clone, Copy)]
pub struct GlobalOptions {
    filter_intrinsics: bool,
    filter_keywords: bool,
    filter_require: bool,
    filter_module_exports: bool,
}

impl Default for GlobalOptions {
    fn default() -> Self {
        Self {
            filter_intrinsics: true,
            filter_keywords: true,
            filter_require: true,
            filter_module_exports: true,
        }
    }
}

/// Lexical scope.
#[derive(Debug)]
pub struct Scope {
    /// Scopes contained by this scope.
    scopes: Vec<Scope>,
    /// Identifiers local to this scope.
    locals: IndexSet<JsWord>,
    /// Identifiers that are references.
    ///
    /// These could be local or global symbols and we need
    /// to walk all parent scopes to detect if a symbol should
    /// be considered global.
    idents: IndexSet<JsWord>,
}

impl Scope {
    fn new(locals: Option<IndexSet<JsWord>>) -> Self {
        Self {
            scopes: Default::default(),
            locals: locals.unwrap_or(Default::default()),
            idents: Default::default(),
        }
    }
}

/// Analyze the scopes for a module.
#[derive(Debug)]
pub struct GlobalAnalysis {
    root: Scope,
    options: GlobalOptions,
}

impl GlobalAnalysis {
    /// Create a scope analysis.
    pub fn new(options: GlobalOptions) -> Self {
        // Setting locals at the root scope allows us to
        // filter out certain symbols from being detected
        // as global.
        let mut locals = IndexSet::new();

        if options.filter_intrinsics {
            for word in INTRINSICS {
                locals.insert(JsWord::from(word));
            }
        }

        if options.filter_module_exports {
            locals.insert(JsWord::from(MODULE));
            locals.insert(JsWord::from(EXPORTS));
        }

        if options.filter_keywords {
            for word in KEYWORDS {
                locals.insert(JsWord::from(word));
            }
        }

        Self {
            root: Scope::new(Some(locals)),
            options,
        }
    }

    /// Compute the global variables.
    pub fn globals(&self) -> IndexSet<JsWord> {
        let mut global_symbols: IndexSet<JsWord> = Default::default();
        self.compute_globals(&self.root, &mut global_symbols, &mut vec![]);
        global_symbols
    }

    fn compute_globals<'a>(
        &self,
        scope: &'a Scope,
        global_symbols: &mut IndexSet<JsWord>,
        locals_stack: &mut Vec<&'a IndexSet<JsWord>>,
    ) {
        locals_stack.push(&scope.locals);

        let mut combined_locals: IndexSet<JsWord> = Default::default();
        for locals in locals_stack.iter() {
            combined_locals = combined_locals.union(locals).cloned().collect();
        }

        let mut diff: IndexSet<JsWord> =
            scope.idents.difference(&combined_locals).cloned().collect();
        'symbols: for sym in diff.drain(..) {
            // Hack to ignore member expressions where the first
            // part of the path matches a local symbol
            for local in &combined_locals {
                let dot_word = format!("{}.", local);
                if sym.starts_with(&dot_word) {
                    continue 'symbols;
                }
            }

            global_symbols.insert(sym.clone());
        }

        for scope in scope.scopes.iter() {
            self.compute_globals(scope, global_symbols, locals_stack);
        }

        locals_stack.pop();
    }
}

struct ScopeBuilder {
    options: GlobalOptions,
}

impl ScopeBuilder {
    fn _visit_stmt(
        &self,
        n: &Stmt,
        scope: &mut Scope,
        locals: Option<IndexSet<JsWord>>,
        merge: bool,
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
                self._visit_stmt(&*n.body, &mut next_scope, None, false);
                scope.scopes.push(next_scope);
            }
            Stmt::While(n) => {
                let mut next_scope = Scope::new(None);
                self._visit_stmt(&*n.body, &mut next_scope, None, false);
                scope.scopes.push(next_scope);
            }
            Stmt::DoWhile(n) => {
                let mut next_scope = Scope::new(None);
                self._visit_stmt(&*n.body, &mut next_scope, None, false);
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

                self._visit_stmt(&*n.body, &mut next_scope, None, false);
                scope.scopes.push(next_scope);
            }
            Stmt::ForIn(n) => {
                let mut next_scope = Scope::new(None);
                self._visit_stmt(&*n.body, &mut next_scope, None, false);
                scope.scopes.push(next_scope);
            }
            Stmt::ForOf(n) => {
                let mut next_scope = Scope::new(None);
                self._visit_stmt(&*n.body, &mut next_scope, None, false);
                scope.scopes.push(next_scope);
            }
            Stmt::Labeled(n) => {
                let mut next_scope = Scope::new(None);
                self._visit_stmt(&*n.body, &mut next_scope, None, false);
                scope.scopes.push(next_scope);
            }
            Stmt::If(n) => {
                let mut next_scope = Scope::new(None);
                self._visit_expr(&*n.test, &mut next_scope);
                self._visit_stmt(&*n.cons, &mut next_scope, None, false);
                scope.scopes.push(next_scope);

                if let Some(ref alt) = n.alt {
                    let mut next_scope = Scope::new(None);
                    self._visit_stmt(&*alt, &mut next_scope, None, false);
                    scope.scopes.push(next_scope);
                }
            }
            Stmt::Try(n) => {
                let block_stmt = Stmt::Block(n.block.clone());
                let mut next_scope = Scope::new(None);
                self._visit_stmt(&block_stmt, &mut next_scope, None, false);
                scope.scopes.push(next_scope);

                if let Some(ref catch_clause) = n.handler {
                    let block_stmt = Stmt::Block(catch_clause.body.clone());
                    let mut next_scope = Scope::new(None);
                    self._visit_stmt(&block_stmt, &mut next_scope, None, false);
                    scope.scopes.push(next_scope);
                }

                if let Some(ref finalizer) = n.finalizer {
                    let block_stmt = Stmt::Block(finalizer.clone());
                    let mut next_scope = Scope::new(None);
                    self._visit_stmt(&block_stmt, &mut next_scope, None, false);
                    scope.scopes.push(next_scope);
                }
            }
            Stmt::Switch(n) => {
                for case in n.cases.iter() {
                    for stmt in case.cons.iter() {
                        let mut next_scope = Scope::new(None);
                        self._visit_stmt(stmt, &mut next_scope, None, false);
                        scope.scopes.push(next_scope);
                    }
                }
            }
            Stmt::Block(n) => {
                if merge {
                    if let Some(merge_locals) = locals {
                        scope.locals = scope
                            .locals
                            .union(&merge_locals)
                            .cloned()
                            .collect();
                    }
                    for stmt in n.stmts.iter() {
                        self._visit_stmt(stmt, scope, None, false);
                    }
                } else {
                    let mut next_scope = Scope::new(locals);
                    for stmt in n.stmts.iter() {
                        self._visit_stmt(stmt, &mut next_scope, None, false);
                    }
                    scope.scopes.push(next_scope);
                }
            }
            Stmt::Return(n) => {
                if let Some(arg) = &n.arg {
                    self._visit_expr(arg, scope);
                }
            }
            Stmt::Throw(n) => {
                self._visit_expr(&*n.arg, scope);
            }
            // Find symbol references which is the list of candidates
            // that may be global (or local) variable references.
            Stmt::Expr(n) => self._visit_expr(&*n.expr, scope),
            _ => {}
        }
    }

    fn _visit_expr(&self, n: &Expr, scope: &mut Scope) {
        match n {
            Expr::Ident(id) => {
                self.insert_ident(id.sym.clone(), scope);
            }
            Expr::PrivateName(n) => {
                self.insert_ident(private_name_prefix(&n.id.sym), scope);
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
                                self.insert_ident(id.sym.clone(), scope);
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
                            false,
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
            Expr::Call(n) => match &n.callee {
                ExprOrSuper::Expr(expr) => {
                    self._visit_expr(expr, scope);
                    if self.options.filter_require {
                        match &**expr {
                            Expr::Ident(id) => {
                                if REQUIRE == id.sym.as_ref() {
                                    scope.idents.remove(&id.sym);
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            },
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
                            self.insert_ident(ident.id.sym.clone(), scope);
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
                if let Some(word) = self.compute_member(n, scope) {
                    self.insert_ident(word, scope);
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
                    if let Some(body) = &n.body {
                        let block_stmt = Stmt::Block(body.clone());
                        self._visit_stmt(
                            &block_stmt,
                            &mut next_scope,
                            None,
                            false,
                        );
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
        mut locals: IndexSet<JsWord>,
    ) {
        // Capture function parameters as locals
        for param in n.params.iter() {
            let mut names = Vec::new();
            pattern_words(&param.pat, &mut names);
            let param_names: IndexSet<_> =
                names.into_iter().map(|n| n.clone()).collect();
            locals = locals.union(&param_names).cloned().collect();
        }

        if let Some(ref body) = n.body {
            let block_stmt = Stmt::Block(body.clone());
            self._visit_stmt(&block_stmt, scope, Some(locals), true);
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

    fn compute_member(&self, n: &MemberExpr, scope: &Scope) -> Option<JsWord> {
        let mut words = Vec::new();
        self._visit_member(n, &mut words, scope);
        if !words.is_empty() {
            let words = words
                .iter()
                .map(|w| w.as_ref().to_string())
                .collect::<Vec<_>>();

            Some(JsWord::from(words.join(".")))
        } else {
            None
        }
    }

    fn _visit_member<'a>(
        &self,
        n: &'a MemberExpr,
        words: &mut Vec<&'a JsWord>,
        scope: &Scope,
    ) {
        let compute_prop = match &n.obj {
            ExprOrSuper::Expr(expr) => {
                match &**expr {
                    Expr::Ident(_) => {
                        // TODO: Detect whether a member expression `obj`
                        // TODO: would be a local but we need to walk the
                        // TODO: parent hierarchy or have a stack of all locals
                        // TODO: to do this so deferring until later.
                        //
                        // TODO: Once this is done then the hack for this when
                        // TODO: computing globals() can be removed.
                        self._visit_member_expr(expr, words);
                        true
                    }
                    Expr::Call(n) => {
                        match &n.callee {
                            ExprOrSuper::Expr(expr) => match &**expr {
                                Expr::Ident(id) => {
                                    words.push(&id.sym);
                                }
                                _ => {}
                            },
                            _ => {}
                        }
                        false
                    }
                    // NOTE: Don't compute properties for Call expressions
                    // NOTE: otherwise we generate for `fetch().then()`
                    _ => false,
                }
            }
            _ => false,
        };

        if self.options.filter_intrinsics && !words.is_empty() {
            if let Some(first) = words.get(0) {
                if INTRINSICS.contains(&first.as_ref()) {
                    words.clear();
                    return;
                }
            }
        }

        if compute_prop && !n.computed {
            match &*n.prop {
                Expr::Member(n) => self._visit_member(n, words, scope),
                _ => self._visit_member_expr(&*n.prop, words),
            }
        }
    }

    fn _visit_member_expr<'a>(&self, n: &'a Expr, words: &mut Vec<&'a JsWord>) {
        match n {
            Expr::Ident(id) => {
                words.push(&id.sym);
            }
            _ => {}
        }
    }

    fn insert_ident(&self, sym: JsWord, scope: &mut Scope) {
        // An earlier version of this performed filtering of
        // intrinsics and keywords here.
        //
        // This can be refactored to just inserting directly now.
        scope.idents.insert(sym);
    }
}

impl Visit for GlobalAnalysis {
    fn visit_module_item(&mut self, n: &ModuleItem, _: &dyn Node) {
        let scope = &mut self.root;
        let builder = ScopeBuilder {
            options: self.options,
        };
        match n {
            ModuleItem::ModuleDecl(decl) => match decl {
                ModuleDecl::Import(import) => {
                    for spec in import.specifiers.iter() {
                        let id = match spec {
                            ImportSpecifier::Named(n) => &n.local.sym,
                            ImportSpecifier::Default(n) => &n.local.sym,
                            ImportSpecifier::Namespace(n) => &n.local.sym,
                        };
                        scope.locals.insert(id.clone());
                    }
                }
                _ => {}
            },
            ModuleItem::Stmt(stmt) => {
                builder._visit_stmt(stmt, scope, None, false)
            }
        }
    }
}

// The JsWord for PrivateName is stripped of the # symbol
// but that would mean that they would incorrectly shadow
// so we restore it.
fn private_name_prefix(word: &JsWord) -> JsWord {
    JsWord::from(format!("#{}", word.as_ref()))
}
