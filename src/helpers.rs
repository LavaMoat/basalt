//! Collection of helper functions.
use swc_atoms::JsWord;
use swc_ecma_ast::*;

const REQUIRE: &str = "require";

/// Find the symbol names in a variable declaration so that we can
/// check for existence in the fixed or live exports map(s).
pub fn var_symbol_words(var: &VarDecl) -> Vec<(&VarDeclarator, Vec<&JsWord>)> {
    var.decls
        .iter()
        .filter(|decl| match &decl.name {
            Pat::Ident(_) => true,
            Pat::Object(_) => true,
            Pat::Array(_) => true,
            Pat::Rest(_) => true,
            Pat::Assign(_) => true,
            _ => false,
        })
        .map(|decl| {
            let mut names = Vec::new();
            pattern_words(&decl.name, &mut names);
            (decl, names)
        })
        .collect::<Vec<_>>()
}

/// Variant of `var_symbol_words()` that maps symbol names to `&str`.
pub fn var_symbol_names(var: &VarDecl) -> Vec<(&VarDeclarator, Vec<&str>)> {
    var_symbol_words(var)
        .into_iter()
        .map(|(decl, words)| {
            (decl, words.into_iter().map(|w| w.as_ref()).collect())
        })
        .collect()
}

/// Recursively fill names with all the symbols in a pattern.
pub fn pattern_words<'a>(pat: &'a Pat, names: &mut Vec<&'a JsWord>) {
    match pat {
        Pat::Ident(binding) => names.push(&binding.id.sym),
        Pat::Object(obj) => {
            for prop in obj.props.iter() {
                match prop {
                    ObjectPatProp::Assign(entry) => {
                        names.push(&entry.key.sym);
                    }
                    ObjectPatProp::KeyValue(entry) => {
                        let will_recurse = match &*entry.value {
                            Pat::Object(_) => true,
                            Pat::Array(_) => true,
                            _ => false,
                        };
                        pattern_words(&*entry.value, names);
                        if !will_recurse {
                            match &entry.key {
                                PropName::Ident(ident) => {
                                    names.push(&ident.sym);
                                }
                                _ => {}
                            }
                        }
                    }
                    ObjectPatProp::Rest(entry) => {
                        pattern_words(&*entry.arg, names);
                    }
                }
            }
        }
        Pat::Array(arr) => {
            for elem in arr.elems.iter() {
                if let Some(ref elem) = elem {
                    pattern_words(elem, names);
                }
            }
        }
        Pat::Rest(rest) => {
            pattern_words(&*rest.arg, names);
        }
        Pat::Assign(assign) => {
            pattern_words(&*assign.left, names);
        }
        _ => {}
    }
}

/// Detect an expression that is a call to `require()`.
///
/// The call must have a single argument and the argument
/// must be a string literal.
///
/// The returned boolean indicates if the require is part of a member
/// expression which means builtin analysis can treat dot access like
/// object destructuring and indicate that this is not a "default import".
pub fn is_require_expr<'a>(n: &'a Expr) -> Option<(&'a JsWord, bool)> {
    match n {
        Expr::Call(call) => {
            return is_require_call(call).map(|o| (o, false));
        }
        Expr::Member(n) => {
            // `require('buffer').Buffer`
            if let ExprOrSuper::Expr(expr) = &n.obj {
                if let Expr::Call(call) = &**expr {
                    return is_require_call(call).map(|o| (o, true));
                }
            }
        }
        _ => {}
    }
    None
}

fn is_require_call<'a>(call: &'a CallExpr) -> Option<&'a JsWord> {
    if call.args.len() == 1 {
        if let ExprOrSuper::Expr(n) = &call.callee {
            if let Expr::Ident(id) = &**n {
                if id.sym.as_ref() == REQUIRE {
                    let arg = call.args.get(0).unwrap();
                    if let Expr::Lit(lit) = &*arg.expr {
                        if let Lit::Str(s) = lit {
                            return Some(&s.value);
                        }
                    }
                }
            }
        }
    }
    None
}

/// Normalize an import specifier removing nested paths.
///
/// This import specifier is transformed to `caniuse-lite`:
///
/// ```text
/// caniuse-lite/data/features/background-img-opts.js
/// ```
///
/// Or the scoped specifier is trasformed to `@babel/runtime`:
///
/// ```text
/// @babel/runtime/helpers/typeof
/// ```
///
pub fn normalize_specifier<S: AsRef<str>>(spec: S) -> String {
    let is_scoped = spec.as_ref().starts_with("@");
    let mut parts: Vec<String> = spec.as_ref().split("/").map(|s| s.into()).collect();
    let mut key: String = spec.as_ref().into();

    // Scoped packages use a single slash delimiter
    if is_scoped {
        if parts.len() > 2 {
            key = format!("{}/{}", parts.remove(0), parts.remove(0));
        }
    // Otherwise any slash denotes a deep path
    } else {
        if parts.len() > 1 {
            key = parts.remove(0);
        }
    }
    key
}
