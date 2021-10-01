use anyhow::Result;

use swc_common::FileName;
use swc_ecma_loader::{resolve::Resolve, resolvers::node::NodeModulesResolver};

use basalt::module::base::module_base_directory;

#[test]
fn module_base_package() -> Result<()> {
    let spec = "core-js";
    let cwd = std::env::current_dir()?;
    let expected =
        Some(cwd.join("node_modules").join("core-js").canonicalize()?);
    let resolver: NodeModulesResolver = Default::default();
    let base_path = cwd.join("tests");
    let base = FileName::Real(base_path);
    let resolved = resolver.resolve(&base, spec)?;
    match &resolved {
        FileName::Real(module_path) => {
            let module_base = module_base_directory(module_path);
            assert_eq!(expected, module_base);
        }
        _ => {}
    }
    Ok(())
}

#[test]
fn module_base_scoped_package() -> Result<()> {
    let spec = "@babel/core";
    let cwd = std::env::current_dir()?;
    let expected = Some(
        cwd.join("node_modules")
            .join("@babel")
            .join("core")
            .canonicalize()?,
    );
    let resolver: NodeModulesResolver = Default::default();
    let base_path = cwd.join("tests");
    let base = FileName::Real(base_path);
    let resolved = resolver.resolve(&base, spec)?;
    match &resolved {
        FileName::Real(module_path) => {
            let module_base = module_base_directory(module_path);
            assert_eq!(expected, module_base);
        }
        _ => {}
    }
    Ok(())
}

#[test]
fn module_base_package_nested_file() -> Result<()> {
    let spec = "core-js/internals/promise-resolve.js";
    let cwd = std::env::current_dir()?;
    let expected =
        Some(cwd.join("node_modules").join("core-js").canonicalize()?);
    let resolver: NodeModulesResolver = Default::default();
    let base_path = cwd.join("tests");
    let base = FileName::Real(base_path);
    let resolved = resolver.resolve(&base, spec)?;
    match &resolved {
        FileName::Real(module_path) => {
            let module_base = module_base_directory(module_path);
            assert_eq!(expected, module_base);
        }
        _ => {}
    }
    Ok(())
}
