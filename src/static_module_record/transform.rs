//! Transform a module to a static module record program.
use std::path::PathBuf;
use std::sync::Arc;

use swc::{
    config::{JscTarget, Options, SourceMapsConfig},
    Compiler, TransformOutput,
};
use swc_common::{
    errors::{emitter::ColorConfig, Handler},
    FileName, SourceMap,
};
use swc_ecma_ast::*;
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax};

use anyhow::Result;

use super::{Generator, Parser as StaticModuleRecordParser};

/// Sources that may be transformed
#[derive(Debug)]
pub enum TransformSource {
    /// Load a file from disc for the transformation.
    File(PathBuf),
    /// Load from a string.
    Str {
        /// The module source.
        content: String,
        /// The file name for the module.
        file_name: String,
    },
}

/// Transform the module file to a program script.
pub fn transform(source: TransformSource) -> Result<TransformOutput> {
    let sm: Arc<SourceMap> = Arc::new(Default::default());
    let handler = Handler::with_tty_emitter(
        ColorConfig::Auto,
        true,
        false,
        Some(sm.clone()),
    );

    let mut options: Options = Default::default();
    options.source_maps = Some(SourceMapsConfig::Bool(true));

    let fm = match source {
        TransformSource::File(path) => sm.load_file(&path)?,
        TransformSource::Str { content, file_name } => sm.new_source_file(
            FileName::Custom(file_name.into()),
            content.into(),
        ),
    };

    let lexer = Lexer::new(
        Syntax::Es(Default::default()),
        JscTarget::Es2020,
        StringInput::from(&*fm),
        None,
    );

    let mut parser = Parser::new_from(lexer);
    for e in parser.take_errors() {
        e.into_diagnostic(&handler).emit();
    }

    let module = parser
        .parse_module()
        .map_err(|e| e.into_diagnostic(&handler).emit())
        .expect("failed to parse module");

    let mut parser = StaticModuleRecordParser::new();
    let meta = parser.parse(&module)?;
    let generator = Generator::new(&meta);
    let compiler = Compiler::new(sm, Arc::new(handler));
    let script = generator
        .create()
        .expect("failed to generate transformed script");
    let program = Program::Script(script);

    let result = compiler.print(
        &program,
        JscTarget::Es2020,
        SourceMapsConfig::Bool(true),
        None,
        false,
    )?;

    Ok(result)
}
