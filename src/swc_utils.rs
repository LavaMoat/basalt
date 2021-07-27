//! Helpers to get a handler, parser, compiler or bundler.
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;

//use swc::Compiler;
use swc_common::{
    errors::{emitter::ColorConfig, Handler},
    FileName, SourceFile, SourceMap,
};
use swc_ecma_ast::Module;
use swc_ecma_parser::{
    lexer::Lexer, EsConfig, JscTarget, Parser, StringInput, Syntax,
};

pub(crate) fn get_handler() -> (Arc<SourceMap>, Handler) {
    let sm: Arc<SourceMap> = Arc::new(Default::default());
    let handler = Handler::with_tty_emitter(
        ColorConfig::Auto,
        true,
        false,
        Some(sm.clone()),
    );
    (sm, handler)
}

pub(crate) fn get_parser<'a>(
    fm: &'a SourceFile,
) -> Parser<Lexer<'a, StringInput<'a>>> {
    let es_config = EsConfig {
        jsx: true,
        ..Default::default()
    };

    let lexer = Lexer::new(
        Syntax::Es(es_config),
        JscTarget::Es2020,
        StringInput::from(fm),
        None,
    );
    Parser::new_from(lexer)
}

/// Parse a module from a file.
pub fn load_file<P: AsRef<Path>>(
    file: P,
) -> Result<(FileName, Arc<SourceMap>, Module)> {
    let (sm, handler) = get_handler();
    let fm = sm.load_file(file.as_ref())?;
    let file_name = fm.name.clone();

    let mut parser = get_parser(&*fm);
    for e in parser.take_errors() {
        e.into_diagnostic(&handler).emit();
    }

    Ok((
        file_name,
        sm,
        parser
            .parse_module()
            .map_err(|e| {
                // Unrecoverable fatal error occurred
                e.into_diagnostic(&handler).emit()
            })
            .expect("Failed to parse module"),
    ))
}

/// Parse a module from a string.
pub fn load_code<S: AsRef<str>>(
    code: S,
    file_name: Option<FileName>,
) -> Result<(FileName, Arc<SourceMap>, Module)> {
    let (sm, handler) = get_handler();
    let fm = sm.new_source_file(
        file_name.unwrap_or(FileName::Custom("unknown.js".to_string())),
        code.as_ref().into(),
    );

    let file_name = fm.name.clone();

    let mut parser = get_parser(&*fm);
    for e in parser.take_errors() {
        e.into_diagnostic(&handler).emit();
    }

    Ok((
        file_name,
        sm,
        parser
            .parse_module()
            .map_err(|e| {
                // Unrecoverable fatal error occurred
                e.into_diagnostic(&handler).emit()
            })
            .expect("Failed to parse module"),
    ))
}

/*
pub(crate) fn get_compiler() -> (Arc<SourceMap>, Arc<Compiler>) {
    let sm: Arc<SourceMap> = Arc::new(Default::default());
    let handler = Handler::with_tty_emitter(
        ColorConfig::Auto,
        true,
        false,
        Some(sm.clone()),
    );
    let compiler =
        Arc::new(swc::Compiler::new(Arc::clone(&sm), Arc::new(handler)));
    (sm, compiler)
}
*/
