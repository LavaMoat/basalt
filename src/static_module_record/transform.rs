//! Transform a module to a static module record program.
use std::path::Path;
use std::sync::Arc;

use swc::{
    config::{JscTarget, Options, SourceMapsConfig},
    Compiler, PassBuilder, TransformOutput,
};
use swc_common::{
    errors::{emitter::ColorConfig, Handler},
    hygiene::Mark,
    FileName, SourceMap, DUMMY_SP,
};
use swc_ecma_ast::*;
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax};
use swc_ecma_visit::{Fold, VisitMut, VisitMutWith};

use anyhow::Result;

use super::{StaticModuleRecord, Generator, Parser as StaticModuleRecordParser};

struct TransformFold<'a> {
    meta: &'a StaticModuleRecord,
    generator: Generator<'a>,
}

impl<'a> Fold for TransformFold<'a> {
    fn fold_program(&mut self, n: Program) -> Program {
        match n {
            Program::Module(module) => {
                let script = self.generator.create()
                    .expect("failed to generate transformed script");
                Program::Script(script)
            }
            _ => panic!("static module record transform must be a module"),
        }
    }
}

/// Perform the static module record transformation.
pub struct Transform;

impl Transform {
    /// Create a new transform.
    pub fn new() -> Self {
        Self {}
    }

    /// Transform the module to a program.
    pub fn transform<P: AsRef<Path>>(
        &self,
        module: P,
    ) -> Result<TransformOutput> {
        let sm: Arc<SourceMap> = Arc::new(Default::default());
        let handler = Handler::with_tty_emitter(
            ColorConfig::Auto,
            true,
            false,
            Some(sm.clone()),
        );

        let mut options: Options = Default::default();
        options.source_maps = Some(SourceMapsConfig::Bool(true));

        let fm = sm.load_file(module.as_ref())?;
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

        let parser = StaticModuleRecordParser::new();
        let meta = parser.parse(&module)?;
        let generator = Generator::new(&meta);
        let mut transform = TransformFold { meta: &meta, generator };

        let compiler = Compiler::new(sm, Arc::new(handler));
        let program = Program::Module(module);

        // NOTE: using a transform `PassBuilder` does not work
        // NOTE: with the `Program` type as `fold_program()` is never
        // NOTE: called on the `Fold` implementation so we use
        // NOTE: this workaround
        let program = transform.fold_program(program);

        let result = compiler.print(
            &program,
            JscTarget::Es2020,
            SourceMapsConfig::Bool(true),
            None,
            false,
        )?;

        Ok(result)
    }
}
