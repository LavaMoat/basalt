//! Analyze the lexical scopes for a module and generate a tree.
//!
//! Once the scope tree is built we can compute globals by doing a
//! depth-first traversal and performing a union of all the locals
//! for each scope into a set, globals are then symbol references
//! that do not exist in the set of all locals.
//!
//! Member expressions with a dot-delimited path only compare using
//! the first word in the path.
//!
//! Does not handle global variables referenced using the `this` keyword
//! as that would require cross-module analysis of the `new` keyword to
//! correctly determine the scope of the the `this` reference. As globals
//! cannot be referenced using `this` in strict mode this is not a major problem.
//!

use swc_atoms::JsWord;
use swc_ecma_ast::*;
use swc_ecma_visit::{Node, Visit};

use indexmap::IndexSet;

use crate::analysis::scope_builder::{Scope, ScopeBuilder, WordOrPath};

// SEE: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects

const REQUIRE: &str = "require";
const IMPORT: &str = "import";
const MODULE: &str = "module";
const EXPORTS: &str = "exports";
const KEYWORDS: [&'static str; 3] = ["undefined", "NaN", "Infinity"];
const GLOBAL_FUNCTIONS: [&'static str; 12] = [
    "eval",
    "uneval",
    "isFinite",
    "isNaN",
    "parseFloat",
    "parseInt",
    "encodeURI",
    "encodeURIComponent",
    "decodeURI",
    "decodeURIComponent",
    // Deprecated
    "escape",
    "unescape",
];

const INTRINSICS: [&'static str; 51] = [
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
    filter_global_functions: bool,
    filter_dynamic_import: bool,
}

impl Default for GlobalOptions {
    fn default() -> Self {
        Self {
            filter_intrinsics: true,
            filter_keywords: true,
            filter_require: true,
            filter_module_exports: true,
            filter_global_functions: true,
            filter_dynamic_import: true,
        }
    }
}

/// Analyze the scopes for a module.
#[derive(Debug)]
pub struct GlobalAnalysis {
    root: Scope,
    options: GlobalOptions,
    builder: ScopeBuilder,
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

        if options.filter_require {
            locals.insert(JsWord::from(REQUIRE));
        }

        if options.filter_dynamic_import {
            locals.insert(JsWord::from(IMPORT));
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

        if options.filter_global_functions {
            for word in GLOBAL_FUNCTIONS {
                locals.insert(JsWord::from(word));
            }
        }

        Self {
            root: Scope::locals(Some(locals)),
            options,
            builder: ScopeBuilder {},
        }
    }

    /// Compute the global variables.
    pub fn compute(&self) -> IndexSet<JsWord> {
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

        // Build up the difference between the sets, cannot use difference()
        // as they are of different types.
        let mut diff: IndexSet<&WordOrPath> = Default::default();
        for ident in scope.idents.iter() {
            let word: JsWord = ident.into();
            if !combined_locals.contains(&word) {
                diff.insert(ident);
            }
        }

        for sym in diff.drain(..) {
            global_symbols.insert(sym.into_path());
        }

        for scope in scope.scopes.iter() {
            self.compute_globals(scope, global_symbols, locals_stack);
        }

        locals_stack.pop();
    }
}

impl Visit for GlobalAnalysis {
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
                        scope.locals.insert(id.clone());
                    }
                }
                _ => {}
            },
            ModuleItem::Stmt(stmt) => {
                self.builder._visit_stmt(stmt, scope, None)
            }
        }
    }
}
