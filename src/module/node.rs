//! Wrapper type for module AST nodes.
//!
//! Encapsulates additional useful information and provides an
//! iterator for resolving and loading dependencies.
//!

use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicU32, Ordering::SeqCst},
    Arc,
};

use anyhow::{anyhow, Context, Result};
use dashmap::DashMap;
use std::lazy::SyncLazy;

use swc_common::{
    comments::SingleThreadedComments, FileName, SourceFile, SourceMap,
};
use swc_ecma_ast::{Module, TargetEnv};
use swc_ecma_dep_graph::{analyze_dependencies, DependencyDescriptor};

use swc_ecma_loader::{resolve::Resolve, resolvers::node::NodeModulesResolver};

use crate::module::dependencies::is_builtin_module;
use crate::swc_utils::load_file;

/// Counter of module ids.
static COUNTER: SyncLazy<AtomicU32> = SyncLazy::new(|| AtomicU32::new(0));

/// Cache of visited modules.
static CACHE: SyncLazy<DashMap<PathBuf, Arc<VisitedModule>>> =
    SyncLazy::new(|| DashMap::new());

/// Get the map of cached modules.
pub fn cached_modules(
) -> &'static SyncLazy<DashMap<PathBuf, Arc<VisitedModule>>> {
    &CACHE
}

/// Stores the data for a visited module dependency.
pub enum VisitedModule {
    /// A Javascript module.
    Module(FileName, ModuleNode),
    /// A JSON module.
    Json(FileName, ModuleNode),
    /// A builtin module.
    Builtin(FileName),
}

/// Represents a visited dependency.
#[derive(Debug)]
pub struct VisitedDependency<'a> {
    /// The dependency specifier.
    pub spec: String,
    /// The file name for the dependency.
    pub file_name: FileName,
    /// Whether this dependency is the last child.
    pub last: bool,
    /// A parsed module for the dependency.
    pub node: &'a Option<&'a ModuleNode>,
    /// The current visitor state.
    pub state: &'a VisitState,
    /// Indicates whether a cycle has been detected.
    pub cycles: Option<&'a FileName>,
}

/// Stores the branch state for the tree printer.
#[derive(Debug)]
pub struct BranchState {
    /// Determine if this node is the last child
    /// of it's parent.
    pub last: bool,
}

/// Represents the visit state for a node iterator.
#[derive(Debug)]
pub struct VisitState {
    /// Stack of branch states.
    pub open: Vec<BranchState>,
    /// Stack of parents when visiting a module graph.
    ///
    /// Used to detect cycles.
    pub parents: Vec<FileName>,
}

/// Parse a module using a cached value when available.
pub fn parse_module<P: AsRef<Path>>(
    file: P,
    resolver: &Box<dyn Resolve>,
    source_map: Arc<SourceMap>,
) -> Result<Arc<VisitedModule>> {
    let buf = file.as_ref().to_path_buf();
    if let Some(entry) = CACHE.get(&buf) {
        let module = entry.value();
        return Ok(module.clone());
    }

    let (file_name, _source_map, module) =
        load_file(file.as_ref(), Some(source_map))?;

    let id = COUNTER.fetch_add(1, SeqCst);
    let comments: SingleThreadedComments = Default::default();
    let mut node = ModuleNode {
        module: Arc::new(module),
        dependencies: None,
        resolved: Default::default(),
        id,
    };
    node.analyze(&comments);
    node.resolve(resolver, &file_name)?;

    // Don't bother walking dependencies that have already
    // been visited.
    //
    // Note that without this performance is terrible as lots
    // of iterations will be performed whilst visiting the dependency
    // graph.
    node.resolved = node
        .resolved
        .into_iter()
        .filter(|(_, file_name)| {
            if let FileName::Real(module_path) = &file_name {
                return CACHE.get(module_path).is_none();
            }
            true
        })
        .collect();

    let module = Arc::new(VisitedModule::Module(file_name, node));
    let entry = CACHE.entry(buf).or_insert(module);
    Ok(entry.value().clone())
}

/// Parse a JSON file as a module.
pub fn parse_json<P: AsRef<Path>>(
    file: P,
    _resolver: &Box<dyn Resolve>,
    source_map: Arc<SourceMap>) -> Result<Arc<VisitedModule>> {

    let buf = file.as_ref().to_path_buf();
    if let Some(entry) = CACHE.get(&buf) {
        let module = entry.value();
        return Ok(module.clone());
    }

    let id = COUNTER.fetch_add(1, SeqCst);
    let source_file = source_map.load_file(file.as_ref())?;
    let file_name = FileName::Real(file.as_ref().to_path_buf());
    let module = load_json_as_module(&source_file)?;

    let node = ModuleNode {
        id,
        module: Arc::new(module),
        dependencies: None,
        resolved: vec![],
    };

    let module = Arc::new(VisitedModule::Json(file_name, node));
    let entry = CACHE.entry(buf).or_insert(module);
    Ok(entry.value().clone())
}

/// Parse a file, analyze dependencies and resolve dependency file paths.
pub fn parse_file<P: AsRef<Path>>(
    file: P,
    resolver: &Box<dyn Resolve>,
    source_map: Arc<SourceMap>,
) -> Result<Arc<VisitedModule>> {
    let extension = file
        .as_ref()
        .extension()
        .map(|s| s.to_string_lossy().to_string());
    if let Some(ref extension) = extension {
        let extension = &extension[..];
        match extension {
            "json" => parse_json(file, resolver, source_map),
            _ => parse_module(file, resolver, source_map),
        }
    } else {
        parse_module(file, resolver, source_map)
    }
}

/// Encapsulates a module and it's dependencies.
#[derive(Debug)]
pub struct ModuleNode {
    /// The module identifier.
    pub id: u32,
    /// The underlying module AST node.
    pub module: Arc<Module>,
    /// The parsed dependencies of this module.
    pub dependencies: Option<Vec<DependencyDescriptor>>,
    /// The resolved paths for the dependencies.
    pub resolved: Vec<(String, FileName)>,
}

impl ModuleNode {
    /// Analyze the dependencies for this module.
    pub fn analyze(&mut self, comments: &SingleThreadedComments) {
        let deps = analyze_dependencies(&self.module, comments);
        self.dependencies = if deps.is_empty() { None } else { Some(deps) };
    }

    /// Resolve the dependencies for this module.
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

    /// Iterate the resolved dependencies of this module and
    /// attempt to load a module for each resolved dependency.
    fn iter<'a>(&'a self, source_map: Arc<SourceMap>) -> NodeIterator<'a> {
        NodeIterator {
            node: self,
            index: 0,
            resolver: Box::new(NodeModulesResolver::new(
                TargetEnv::Node,
                Default::default(),
            )),
            source_map,
        }
    }

    /// Visit all dependencies of this node recursively.
    pub fn visit<F>(
        &self,
        source_map: Arc<SourceMap>,
        callback: &mut F,
    ) -> Result<()>
    where
        F: FnMut(VisitedDependency) -> Result<()>,
    {
        let mut state = VisitState {
            open: Vec::new(),
            parents: Vec::new(),
        };
        self.visit_all(self, &mut state, source_map, callback)
    }

    fn visit_all<F>(
        &self,
        node: &ModuleNode,
        state: &mut VisitState,
        source_map: Arc<SourceMap>,
        callback: &mut F,
    ) -> Result<()>
    where
        F: FnMut(VisitedDependency) -> Result<()>,
    {
        state.open.push(BranchState { last: false });

        for res in node.iter(Arc::clone(&source_map)) {
            let (i, spec, parsed) = res?;
            let last = i == (node.resolved.len() - 1);
            state.open.last_mut().unwrap().last = last;

            let (file_name, dep) = match &*parsed {
                VisitedModule::Module(file_name, dep) => (file_name, Some(dep)),
                VisitedModule::Json(file_name, dep) => (file_name, Some(dep)),
                VisitedModule::Builtin(file_name) => (file_name, None),
            };

            //println!("Visiting {:#?}", file_name);

            let cycles = state.parents.iter().find(|p| p == &file_name);

            let dependency = VisitedDependency {
                spec,
                file_name: file_name.clone(),
                last: i == node.resolved.len() - 1,
                node: &dep,
                state,
                cycles,
            };

            callback.call_mut((dependency,))?;

            if cycles.is_some() {
                continue;
            }

            if let Some(dep) = dep {
                if !dep.resolved.is_empty() {
                    state.parents.push(file_name.clone());
                    self.visit_all(
                        &dep,
                        state,
                        Arc::clone(&source_map),
                        callback,
                    )?;
                    state.parents.pop();
                }
            }
        }

        state.open.pop();

        Ok(())
    }
}

/// Iterate the resolved dependencies of a module node.
pub struct NodeIterator<'a> {
    node: &'a ModuleNode,
    resolver: Box<dyn Resolve>,
    index: usize,
    source_map: Arc<SourceMap>,
}

impl<'a> Iterator for NodeIterator<'a> {
    type Item = Result<(usize, String, Arc<VisitedModule>)>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.node.resolved.len() {
            return None;
        }

        if let Some(resolved) = self.node.resolved.get(self.index) {
            self.index += 1;

            match &resolved.1 {
                FileName::Real(file_name) => {
                    return match parse_file(
                        file_name,
                        &self.resolver,
                        Arc::clone(&self.source_map),
                    ) {
                        Ok(parsed) => Some(Ok((
                            self.index - 1,
                            resolved.0.clone(),
                            parsed,
                        ))),
                        Err(e) => Some(Err(anyhow!(e))),
                    };
                }
                FileName::Custom(file_name) => {
                    if is_builtin_module(file_name) {
                        let builtin_module = Arc::new(VisitedModule::Builtin(
                            resolved.1.clone(),
                        ));
                        return Some(Ok((
                            self.index - 1,
                            resolved.0.clone(),
                            builtin_module,
                        )));
                    }
                }
                _ => {}
            }
        }
        None
    }
}

/// Load a JSON file and convert to a CJS module.
fn load_json_as_module(fm: &Arc<SourceFile>) -> Result<Module> {
    use swc_common::{input::SourceFileInput, DUMMY_SP};
    use swc_ecma_ast::{EsVersion, *};
    use swc_ecma_parser::{lexer::Lexer, Parser, Syntax};

    let lexer = Lexer::new(
        Syntax::default(),
        EsVersion::Es2020,
        SourceFileInput::from(&**fm),
        None,
    );
    let mut parser = Parser::new_from(lexer);
    let expr = parser.parse_expr().map_err(|err| {
        anyhow!("failed parse json as javascript object: {:#?}", err)
    })?;

    let export = ModuleItem::Stmt(Stmt::Expr(ExprStmt {
        span: DUMMY_SP,
        expr: Box::new(Expr::Assign(AssignExpr {
            span: DUMMY_SP,
            op: op!("="),
            left: PatOrExpr::Expr(Box::new(Expr::Member(MemberExpr {
                span: DUMMY_SP,
                obj: ExprOrSuper::Expr(Box::new(Expr::Ident(Ident::new(
                    "module".into(),
                    DUMMY_SP,
                )))),
                prop: Box::new(Expr::Ident(Ident::new(
                    "exports".into(),
                    DUMMY_SP,
                ))),
                computed: false,
            }))),
            right: expr,
        })),
    }));

    Ok(Module {
        span: DUMMY_SP,
        body: vec![export],
        shebang: None,
    })
}
