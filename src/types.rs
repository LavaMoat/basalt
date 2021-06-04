use anyhow::{Context, Result};
use swc_bundler::Resolve;
use swc_common::{comments::SingleThreadedComments, FileName, SourceMap};
use swc_ecma_ast::Module;
use swc_ecma_dep_graph::{analyze_dependencies, DependencyDescriptor};

#[derive(Debug)]
pub struct ModuleNode {
    pub module: Module,
    pub dependencies: Option<Vec<DependencyDescriptor>>,
    pub resolved: Vec<(String, FileName)>,
}

impl ModuleNode {
    pub fn analyze(
        &mut self,
        source_map: &SourceMap,
        comments: &SingleThreadedComments,
    ) {
        let deps = analyze_dependencies(&self.module, source_map, comments);
        self.dependencies = if deps.is_empty() { None } else { Some(deps) };
    }

    pub fn resolve(
        &mut self,
        resolver: &Box<dyn Resolve>,
        base: &FileName,
    ) -> Result<()> {
        if let Some(deps) = &self.dependencies {
            for dep in deps {
                let spec = format!("{}", dep.specifier);
                let file_name = resolver.resolve(base, &spec).context(
                    format!("Failed to resolve module for {}", &spec),
                )?;
                self.resolved.push((spec, file_name));
            }
        }
        Ok(())
    }
}

impl From<Module> for ModuleNode {
    fn from(module: Module) -> Self {
        ModuleNode {
            module,
            dependencies: None,
            resolved: Vec::new(),
        }
    }
}
