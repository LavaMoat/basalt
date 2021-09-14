//! Collection of helper functions.
use swc_atoms::JsWord;
use swc_ecma_ast::*;

/// Common Javascript module.
pub const MODULE: &str = "module";
/// Common Javascript exports.
pub const EXPORTS: &str = "exports";

/// Walk a variable declaration and find all symbols.
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
    let mut parts: Vec<String> =
        spec.as_ref().split("/").map(|s| s.into()).collect();
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

/// Determine if an expression refers to CJS module exports.
///
/// Comparison is for plain `exports` and `module.exports`.
pub fn is_module_exports(n: &PatOrExpr) -> bool {
    match n {
        PatOrExpr::Pat(pat) => match &**pat {
            Pat::Ident(ident) => {
                return ident.id.sym.as_ref() == EXPORTS;
            }
            Pat::Expr(expr) => match &**expr {
                Expr::Member(n) => {
                    if let (ExprOrSuper::Expr(expr), Expr::Ident(prop)) =
                        (&n.obj, &*n.prop)
                    {
                        if let Expr::Ident(obj) = &**expr {
                            return obj.as_ref() == MODULE
                                && prop.as_ref() == EXPORTS;
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        },
        _ => {}
    }
    false
}
