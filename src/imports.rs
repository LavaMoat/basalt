//! Helper to extract imports from a module
use swc::Compiler;
use swc_common::{
    FileName, Mark, SyntaxContext,
};
use swc_atoms::JsWord;
use swc_ecma_ast::Module;
use swc_bundler_analysis::{
    handler::Handler,
    import::{ImportHandler, RawImports},
    id::ModuleId,
};
use spack::resolvers::NodeResolver;
use swc_bundler::Resolve;
use swc_ecma_visit::VisitMutWith;
use swc_bundler::bundler::scope::Scope;

pub struct ImportExtractor {
    scope: Scope,
    resolver: Box<dyn Resolve>,
    require: bool,
}

impl ImportExtractor {

    pub fn new(require: bool) -> Self {
        Self {
            scope: Default::default(),
            resolver: Box::new(NodeResolver::new()),
            require,
        }
    }

    /// This method de-globs imports if possible and colorizes imported values.
    pub(crate) fn extract_import_info(
        &self,
        compiler: &Compiler,
        path: &FileName,
        module: &mut Module,
        module_local_mark: Mark,
    ) -> RawImports {
        compiler.run(|| {
            let mut v = ImportHandler {
                module_ctxt: SyntaxContext::empty().apply_mark(module_local_mark),
                path,
                handler: &self,
                top_level: false,
                info: Default::default(),
                usages: Default::default(),
                imported_idents: Default::default(),
                deglob_phase: false,
                idents_to_deglob: Default::default(),
                in_obj_of_member: false,
            };
            module.body.visit_mut_with(&mut v);
            v.deglob_phase = true;
            module.body.visit_mut_with(&mut v);

            v.info
        })
    }
}

impl Handler for &ImportExtractor {
    fn is_external_module(&self, module_specifier: &JsWord) -> bool {
        //self.config.external_modules.contains(module_specifier)
        false
    }

    fn resolve(&self, base: &FileName, src: &JsWord) -> Option<FileName> {
        self.resolver.resolve(base, src).ok()
    }

    fn get_module_info(&self, path: &FileName) -> (ModuleId, Mark, Mark) {
        self.scope.module_id_gen.gen(path)
    }

    fn supports_cjs(&self) -> bool {
        self.require
    }

    fn mark_as_cjs(&self, id: ModuleId) {
        self.scope.mark_as_cjs(id)
    }

    fn mark_as_wrapping_required(&self, id: ModuleId) {
        self.scope.mark_as_wrapping_required(id)
    }
}
