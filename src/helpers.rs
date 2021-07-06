//! Collection of helper functions.
use swc_atoms::JsWord;
use swc_ecma_ast::*;

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

fn pattern_words<'a>(pat: &'a Pat, names: &mut Vec<&'a JsWord>) {
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
        _ => {}
    }
}

