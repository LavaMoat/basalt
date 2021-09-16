//! Helpers to get a handler, parser, compiler or bundler.
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;

use swc::{
    config::{JscTarget, SourceMapsConfig},
    Compiler, TransformOutput,
};
use swc_common::{
    errors::{emitter::ColorConfig, Handler},
    FileName, SourceFile, SourceMap,
};
use swc_ecma_ast::Module;
use swc_ecma_codegen::Node;
use swc_ecma_parser::{lexer::Lexer, EsConfig, Parser, StringInput, Syntax};

use swc::IdentCollector;
use swc_ecma_visit::VisitWith;

pub(crate) fn get_handler(
    source_map: Option<Arc<SourceMap>>,
) -> (Arc<SourceMap>, Handler) {
    let sm: Arc<SourceMap> =
        source_map.unwrap_or_else(|| Arc::new(Default::default()));
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
        dynamic_import: true,
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
    source_map: Option<Arc<SourceMap>>,
) -> Result<(FileName, Arc<SourceMap>, Module)> {
    let (sm, handler) = get_handler(source_map);
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
            .map_err(|e| e.into_diagnostic(&handler).emit())
            .expect("Failed to parse module"),
    ))
}

/// Parse a module from a string.
pub fn load_code<S: AsRef<str>>(
    code: S,
    file_name: Option<FileName>,
    source_map: Option<Arc<SourceMap>>,
) -> Result<(FileName, Arc<SourceMap>, Module)> {
    let (sm, handler) = get_handler(source_map);
    let fm = sm.new_source_file(
        file_name.unwrap_or(FileName::Anon),
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
            .map_err(|e| e.into_diagnostic(&handler).emit())
            .expect("Failed to parse module"),
    ))
}

// NOTE: The signature for Compiler.print() changes a lot
// NOTE: so we prefer to wrap it to simplify updates.

/// Print a node.
pub fn print<T>(
    node: &T,
    source_map: Arc<SourceMap>,
    source_file_name: Option<&str>,
    output_path: Option<PathBuf>,
    source_maps_config: SourceMapsConfig,
) -> Result<TransformOutput>
where
    T: Node + VisitWith<IdentCollector>,
{
    //node: &T,
    //source_file_name: Option<&str>,
    //output_path: Option<PathBuf>,
    //inline_sources_content: bool,
    //target: JscTarget,
    //source_map: SourceMapsConfig,
    //source_map_names: &[JsWord],
    //orig: Option<&sourcemap::SourceMap>,
    //minify: bool,
    //preserve_comments: Option<BoolOrObject<JsMinifyCommentOption>>,

    //let sm: Arc<SourceMap> = Arc::new(Default::default());
    let compiler = Compiler::new(source_map);
    compiler.print(
        node,
        source_file_name,
        output_path,
        false,
        JscTarget::Es2020,
        source_maps_config,
        &[],
        None,
        false,
        None,
    )
}
