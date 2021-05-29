//! Helpers to get a compiler and bundler.
use std::sync::Arc;

use anyhow::{Error, Result};

use swc::{config::SourceMapsConfig, Compiler, TransformOutput};
use swc_atoms::js_word;
use swc_atoms::JsWord;
use swc_bundler::{BundleKind, Bundler, Load, ModuleRecord, Resolve};
use swc_common::{
    errors::{Handler, emitter::{ColorConfig}},
    SourceMap, Globals, Span,
};

use swc_ecma_ast::{
    Bool, Expr, ExprOrSuper, Ident, KeyValueProp, Lit, MemberExpr, MetaPropExpr, PropName, Str,
};

use spack::{loaders::swc::SwcLoader, resolvers::NodeResolver};

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
                        obj: ExprOrSuper::Expr(Box::new(Expr::MetaProp(MetaPropExpr {
                            meta: Ident::new(js_word!("import"), span),
                            prop: Ident::new(js_word!("meta"), span),
                        }))),
                        prop: Box::new(Expr::Ident(Ident::new(js_word!("main"), span))),
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
    options: swc::config::Options,
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

pub(crate) fn get_compiler() -> Compiler {
    let sm: Arc<SourceMap> = Arc::new(Default::default());
    let handler =
        Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(sm.clone()));
    swc::Compiler::new(sm, Arc::new(handler))
}
