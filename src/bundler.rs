//! Helpers to get a handler, parser, compiler or bundler.
use std::sync::Arc;
use std::path::Path;

use anyhow::{anyhow, Error, Result};

use swc::Compiler;
use swc_atoms::js_word;
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, input::TokensInput};
use swc_atoms::JsWord;
use swc_bundler::{Bundler, Load, ModuleRecord, Resolve};
use swc_common::{
    errors::{emitter::ColorConfig, Handler},
    Globals, SourceMap, Span, FileName, SourceFile,
};

use swc_ecma_ast::{
    Bool, Expr, ExprOrSuper, Ident, KeyValueProp, Lit, MemberExpr,
    MetaPropExpr, PropName, Str, Module,
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

pub(crate) fn get_compiler() -> (Arc<SourceMap>, Arc<Compiler>) {
    let (sm, handler) = get_handler();
    let compiler =
        Arc::new(swc::Compiler::new(Arc::clone(&sm), Arc::new(handler)));
    (sm, compiler)
}

pub(crate) fn get_parser<'a>(fm: &'a SourceFile) -> Parser<Lexer<'a, StringInput<'a>>> {
    let lexer = Lexer::new(
        // We want to parse ecmascript
        Syntax::Es(Default::default()),
        // JscTarget defaults to es5
        Default::default(),
        StringInput::from(fm),
        None,
    );
    Parser::new_from(lexer)
}

// Parse string code, useful for quick debugging.
pub(crate) fn parse(code: &str, file_name: &str) -> Result<Module> {
    let (sm, handler) = get_handler();

    let fm = sm.new_source_file(
        FileName::Custom(file_name.into()),
        code.into(),
    );

    let mut parser = get_parser(&*fm);
    for e in parser.take_errors() {
        e.into_diagnostic(&handler).emit();
    }

    Ok(parser.parse_module()
        .map_err(|e| {
            // Unrecoverable fatal error occurred
            e.into_diagnostic(&handler).emit()
        })
        .expect("Failed to parse module"))
}

// Parse a module from a file.
pub(crate) fn load_file<P: AsRef<Path>>(file: P) -> Result<Module> {
    let (sm, handler) = get_handler();
    let fm = sm.load_file(file.as_ref())?;

    let mut parser = get_parser(&*fm);
    for e in parser.take_errors() {
        e.into_diagnostic(&handler).emit();
    }

    Ok(parser
        .parse_module()
        .map_err(|e| {
            // Unrecoverable fatal error occurred
            e.into_diagnostic(&handler).emit()
        })
        .expect("Failed to parse module"))
}

struct Hook;

impl swc_bundler::Hook for Hook {
    fn get_import_meta_props(
        &self,
        span: Span,
        module_record: &ModuleRecord,
    ) -> Result<Vec<KeyValueProp>, Error> {
        Ok(vec![
            KeyValueProp {
                key: PropName::Ident(Ident::new(js_word!("url"), span)),
                value: Box::new(Expr::Lit(Lit::Str(Str {
                    span,
                    value: module_record.file_name.to_string().into(),
                    has_escape: false,
                    kind: Default::default(),
                }))),
            },
            KeyValueProp {
                key: PropName::Ident(Ident::new(js_word!("main"), span)),
                value: Box::new(if module_record.is_entry {
                    Expr::Member(MemberExpr {
                        span,
                        obj: ExprOrSuper::Expr(Box::new(Expr::MetaProp(
                            MetaPropExpr {
                                meta: Ident::new(js_word!("import"), span),
                                prop: Ident::new(js_word!("meta"), span),
                            },
                        ))),
                        prop: Box::new(Expr::Ident(Ident::new(
                            js_word!("main"),
                            span,
                        ))),
                        computed: false,
                    })
                } else {
                    Expr::Lit(Lit::Bool(Bool { span, value: false }))
                }),
            },
        ])
    }
}

pub(crate) fn get_bundler<'a>(
    compiler: Arc<swc::Compiler>,
    globals: &'a Globals,
    loader: &'a Box<dyn Load>,
    resolver: &'a Box<dyn Resolve>,
) -> Bundler<'a, &'a Box<dyn Load>, &'a Box<dyn Resolve>> {
    Bundler::new(
        globals,
        compiler.cm.clone(),
        loader,
        resolver,
        swc_bundler::Config {
            require: true,
            external_modules: vec![
                "assert",
                "buffer",
                "child_process",
                "console",
                "cluster",
                "crypto",
                "dgram",
                "dns",
                "events",
                "fs",
                "http",
                "http2",
                "https",
                "net",
                "os",
                "path",
                "perf_hooks",
                "process",
                "querystring",
                "readline",
                "repl",
                "stream",
                "string_decoder",
                "timers",
                "tls",
                "tty",
                "url",
                "util",
                "v8",
                "vm",
                "wasi",
                "worker",
                "zlib",
            ]
            .into_iter()
            .map(JsWord::from)
            //.chain(
            //self.config
            //.static_items
            //.config
            //.extenal_modules
            //.iter()
            //.cloned(),
            //)
            .collect(),
            ..Default::default()
        },
        Box::new(Hook),
    )
}

