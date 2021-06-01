use swc_bundler::{
    bundler::load::{Source, Specifier},
};

// Hack to de-duplicate the import specifiers.
//
// SEE: https://github.com/swc-project/swc/discussions/1768
pub(crate) fn dedupe_import_specifiers(input: &mut Vec<(Source, Vec<Specifier>)>) {
    let mut already_seen = vec![];
    input.retain(|(source, _)| {
        match already_seen.contains(&source.src.value) {
            true => false,
            _ => {
                already_seen.push(source.src.value.clone());
                true
            }
        }
    })
}

