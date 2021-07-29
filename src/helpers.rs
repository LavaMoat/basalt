//! Collection of helper functions.
use swc_atoms::JsWord;
use swc_ecma_ast::*;

const REQUIRE: &str = "require";

/// Member expressions have the furthest left of the path
/// as the deepest expression which is awkward for analysis
/// so we walk all member expressions and invert them.
pub fn member_expr_words(n: &MemberExpr) -> Vec<&JsWord> {
    let mut members = Vec::new();
    walk_member_expr(n, &mut members);
    members
}

fn walk_member_expr<'a>(n: &'a MemberExpr, members: &mut Vec<&'a JsWord>) {
    if let ExprOrSuper::Expr(n) = &n.obj {
        match &**n {
            Expr::Ident(id) => {
                members.push(&id.sym);
            }
            Expr::Member(n) => {
                walk_member_expr(n, members);
            }
            _ => walk_nested_member_expr(n, members),
        }
    }

    if n.computed {
        return;
    };

    match &*n.prop {
        Expr::Ident(id) => {
            members.push(&id.sym);
        }
        _ => walk_nested_member_expr(&*n.prop, members),
    }
}

fn walk_nested_member_expr<'a>(n: &'a Expr, members: &mut Vec<&'a JsWord>) {
    // TODO: implement this correctly
    match n {
        Expr::Member(n) => {
            walk_member_expr(n, members);
        }
        _ => {}
    }
}

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
        _ => {}
    }
}

/// Detect an expression that is a call to `require()`.
///
/// The call must have a single argument and the argument
/// must be a string literal.
pub fn is_require_expr<'a>(n: &'a Expr) -> Option<&'a JsWord> {
    match n {
        Expr::Call(call) => {
            return is_require_call(call);
        }
        Expr::Member(n) => {
            // `require('buffer').Buffer`
            if let ExprOrSuper::Expr(expr) = &n.obj {
                if let Expr::Call(call) = &**expr {
                    return is_require_call(call);
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
