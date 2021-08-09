//! Visitors and helpers for module analysis.

use indexmap::IndexSet;
use swc_atoms::JsWord;

pub mod builtin;
pub mod dynamic_import;
pub mod globals_scope;
pub mod member_expr;
pub mod module_exports;
pub mod scope_builder;

/// Join the keys of a set into a single dot-delimited word.
pub fn join_keys(set: IndexSet<Vec<JsWord>>) -> IndexSet<JsWord> {
    set.iter().map(|words| join_words(words)).collect()
}

/// Join the words into a single dot-delimited word.
pub fn join_words(words: &Vec<JsWord>) -> JsWord {
    let words: Vec<String> =
        words.into_iter().map(|w| w.as_ref().to_string()).collect();
    JsWord::from(words.join("."))
}

/// Flatten the computed symbol list so that deep properties are
/// accumulated with the parent reference.
///
/// For example, if we have `Buffer` and `Buffer.alloc` the `Buffer.alloc`
/// entry is removed and we defer to the parent `Buffer`.
pub fn flatten(set: IndexSet<Vec<JsWord>>) -> IndexSet<Vec<JsWord>> {
    let compare = set.clone();
    set.into_iter()
        .filter(|k| {
            for key in compare.iter() {
                if key.len() < k.len() {
                    if k.starts_with(&key) {
                        return false;
                    }
                }
            }
            true
        })
        .collect()
}
