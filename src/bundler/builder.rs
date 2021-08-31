//! Builder for creating bundles.

use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{bail, Context, Result};

use swc_common::{FileName, SourceMap, DUMMY_SP};
use swc_ecma_ast::*;
use swc_ecma_loader::{resolve::Resolve, resolvers::node::NodeModulesResolver};
use swc_ecma_transforms_base::ext::MapWithMut;
use swc_ecma_visit::{Fold, FoldWith};

use serde::Serialize;

use crate::{
    module::base::module_base_directory,
    policy::{Merge, Policy},
    swc_utils::load_file,
};

use super::{
    loader::load_modules,
    serializer::{Serializer, Value},
};

const RUNTIME_PACKAGE: &str = "@lavamoat/lavapack";
const MODULES: &str = "__modules__";
const ENTRY_POINTS: &str = "__entryPoints__";
const POLICY: &str = "__policy__";
const LAVA_PACK: &str = "LavaPack";
const LOAD_BUNDLE: &str = "loadBundle";

pub(crate) struct BundleBuilder {
    policy: Policy,
    program: Program,
    source_map: Arc<SourceMap>,
    resolver: Box<dyn Resolve>,
}

impl BundleBuilder {
    /// Create a bundle builder.
    pub fn new() -> Self {
        let source_map: Arc<SourceMap> = Arc::new(Default::default());
        let program = Program::Script(Script {
            span: DUMMY_SP,
            body: vec![],
            shebang: None,
        });
        let resolver: Box<dyn Resolve> =
            Box::new(NodeModulesResolver::default());

        Self {
            policy: Default::default(),
            program,
            source_map,
            resolver,
        }
    }

    /// Load policy files.
    pub fn load_policy_files(mut self, policy: &Vec<PathBuf>) -> Result<Self> {
        for file in policy {
            let f = File::open(file).context(format!(
                "Unable to open policy file {}",
                file.display()
            ))?;
            let reader = BufReader::new(f);
            let mut policy: Policy = serde_json::from_reader(reader).context(
                format!("Failed to parse JSON in {}", file.display()),
            )?;
            self.policy.merge(&mut policy);
        }
        Ok(self)
    }

    /// Fold into a single program.
    pub fn fold(mut self, entry: PathBuf) -> Result<Self> {
        // Load and inject the runtime
        let module = self.load_runtime_module()?;
        let mut runtime_module = RuntimeModule { module };
        self.program = self.program.fold_children_with(&mut runtime_module);

        let list =
            load_modules(entry, Arc::clone(&self.source_map), &self.resolver)?;

        println!("Got modules list {:#?}", list.modules.len());

        // TODO: build modules data structure!
        let mut modules_decl = ModulesDecl {};
        self.program = self.program.fold_children_with(&mut modules_decl);

        // TODO: collect entry points from CLI args
        let mut entries_decl = EntryPointsDecl {};
        self.program = self.program.fold_children_with(&mut entries_decl);

        // Serialize and inject the computed policy
        let policy_expr =
            PolicyDecl::build_policy(std::mem::take(&mut self.policy))?;
        let mut policy_decl = PolicyDecl { expr: policy_expr };
        self.program = self.program.fold_children_with(&mut policy_decl);

        // Initialize the bundle
        //
        // LavaPack.loadBundle(__modules__, __entryPoints__, __policy__)
        //
        let mut bundle_call = LoadBundleCall {};
        self.program = self.program.fold_children_with(&mut bundle_call);

        let mut iife = Iife {};
        self.program = self.program.fold_children_with(&mut iife);

        // [123, {'./util.js': 456 }, function(){ module.exports = 42 }, { package: '<root>' }]

        Ok(self)
    }

    /// Finalize the bundled program.
    pub fn finalize(self) -> (Program, Arc<SourceMap>) {
        (self.program, self.source_map)
    }

    /// Load the runtime module.
    fn load_runtime_module(&self) -> Result<Module> {
        let base_dir = FileName::Real(std::env::current_dir()?);
        let runtime_lib = self
            .resolver
            .resolve(&base_dir, RUNTIME_PACKAGE)
            .context(format!(
                "could not find {}, ensure it has been installed",
                RUNTIME_PACKAGE
            ))?;

        let lib_index = match runtime_lib {
            FileName::Real(path) => path,
            _ => bail!("runtime library must be a real path"),
        };
        let package_dir = module_base_directory(&lib_index).unwrap();
        let runtime_file = package_dir.join("src").join("runtime.js");

        if !runtime_file.is_file() {
            bail!("runtime {} is not a file", runtime_file.display());
        }

        let (_, _, module) =
            load_file(&runtime_file, Some(Arc::clone(&self.source_map)))?;

        Ok(module)
    }
}

/// Inject the LavaPack runtime.
struct RuntimeModule {
    module: Module,
}

impl Fold for RuntimeModule {
    fn fold_script(&mut self, mut n: Script) -> Script {
        let module = self.module.take();
        for item in module.body {
            match item {
                ModuleItem::Stmt(stmt) => n.body.push(stmt),
                _ => {}
            }
        }
        n
    }
}

/// Inject the module definition data structure.
struct ModulesDecl;

impl Fold for ModulesDecl {
    fn fold_script(&mut self, mut n: Script) -> Script {
        let decl = Stmt::Decl(Decl::Var(VarDecl {
            span: DUMMY_SP,
            kind: VarDeclKind::Const,
            declare: false,
            decls: vec![VarDeclarator {
                span: DUMMY_SP,
                definite: false,
                name: Pat::Ident(BindingIdent {
                    id: Ident {
                        span: DUMMY_SP,
                        optional: false,
                        sym: MODULES.into(),
                    },
                    type_ann: None,
                }),
                init: Some(Box::new(Expr::Lit(Lit::Null(Null {
                    span: DUMMY_SP,
                })))),
            }],
        }));
        n.body.push(decl);
        n
    }
}

/// Inject the entry points data structure.
struct EntryPointsDecl;

impl Fold for EntryPointsDecl {
    fn fold_script(&mut self, mut n: Script) -> Script {
        let decl = Stmt::Decl(Decl::Var(VarDecl {
            span: DUMMY_SP,
            kind: VarDeclKind::Const,
            declare: false,
            decls: vec![VarDeclarator {
                span: DUMMY_SP,
                definite: false,
                name: Pat::Ident(BindingIdent {
                    id: Ident {
                        span: DUMMY_SP,
                        optional: false,
                        sym: ENTRY_POINTS.into(),
                    },
                    type_ann: None,
                }),
                init: Some(Box::new(Expr::Lit(Lit::Null(Null {
                    span: DUMMY_SP,
                })))),
            }],
        }));
        n.body.push(decl);
        n
    }
}

/// Inject the policy definition.
struct PolicyDecl {
    expr: Expr,
}

impl PolicyDecl {
    fn build_policy(policy: Policy) -> Result<Expr> {
        let mut serializer = Serializer {};
        let value = policy.serialize(&mut serializer)?;
        if let Value::Object(obj) = value {
            return Ok(Expr::Object(obj));
        }
        unreachable!("serialized policy must be an object");
    }
}

impl Fold for PolicyDecl {
    fn fold_script(&mut self, mut n: Script) -> Script {
        let decl = Stmt::Decl(Decl::Var(VarDecl {
            span: DUMMY_SP,
            kind: VarDeclKind::Const,
            declare: false,
            decls: vec![VarDeclarator {
                span: DUMMY_SP,
                definite: false,
                name: Pat::Ident(BindingIdent {
                    id: Ident {
                        span: DUMMY_SP,
                        optional: false,
                        sym: POLICY.into(),
                    },
                    type_ann: None,
                }),
                init: Some(Box::new(self.expr.take())),
            }],
        }));
        n.body.push(decl);
        n
    }
}

/// Call LavaPack.loadBundle().
struct LoadBundleCall;

impl Fold for LoadBundleCall {
    fn fold_script(&mut self, mut n: Script) -> Script {
        let stmt = Stmt::Expr(ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(Expr::Call(CallExpr {
                span: DUMMY_SP,
                callee: ExprOrSuper::Expr(Box::new(Expr::Member(MemberExpr {
                    span: DUMMY_SP,
                    computed: false,
                    obj: ExprOrSuper::Expr(Box::new(Expr::Ident(Ident {
                        span: DUMMY_SP,
                        sym: LAVA_PACK.into(),
                        optional: false,
                    }))),
                    prop: Box::new(Expr::Ident(Ident {
                        span: DUMMY_SP,
                        sym: LOAD_BUNDLE.into(),
                        optional: false,
                    })),
                }))),
                args: vec![
                    ExprOrSpread {
                        spread: None,
                        expr: Box::new(Expr::Ident(Ident {
                            span: DUMMY_SP,
                            sym: MODULES.into(),
                            optional: false,
                        })),
                    },
                    ExprOrSpread {
                        spread: None,
                        expr: Box::new(Expr::Ident(Ident {
                            span: DUMMY_SP,
                            sym: ENTRY_POINTS.into(),
                            optional: false,
                        })),
                    },
                    ExprOrSpread {
                        spread: None,
                        expr: Box::new(Expr::Ident(Ident {
                            span: DUMMY_SP,
                            sym: POLICY.into(),
                            optional: false,
                        })),
                    },
                ],
                type_args: None,
            })),
        });
        n.body.push(stmt);
        n
    }
}

/// Wrap everything in an IIFE.
struct Iife;
impl Fold for Iife {
    fn fold_script(&mut self, mut n: Script) -> Script {
        let iife = Stmt::Expr(ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(Expr::Unary(UnaryExpr {
                span: DUMMY_SP,
                op: UnaryOp::Void,
                arg: Box::new(Expr::Call(CallExpr {
                    span: DUMMY_SP,
                    callee: ExprOrSuper::Expr(Box::new(Expr::Fn(FnExpr {
                        ident: None,
                        function: Function {
                            params: vec![],
                            body: Some(BlockStmt {
                                span: DUMMY_SP,
                                stmts: n.body.take(),
                            }),
                            decorators: vec![],
                            span: DUMMY_SP,
                            is_generator: false,
                            is_async: false,
                            type_params: None,
                            return_type: None,
                        },
                    }))),
                    args: vec![],
                    type_args: None,
                })),
            })),
        });

        n.body = vec![iife];
        n
    }
}
