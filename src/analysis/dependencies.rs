//! Detect built in packages for node declared as ESM imports
//! or CommonJS require expressions.
//!
//! Run `node -p "require('module').builtinModules"` to generate the
//! list of built in packages for a version of node.
//!

use swc_common::{comments::SingleThreadedComments, FileName, SourceMap};
use swc_ecma_ast::Module;
use swc_ecma_dep_graph::{analyze_dependencies, DependencyDescriptor};

/// List of built in packages for latest stable node with LTS (node@16).
pub const NODE_LATEST_STABLE: &'static [&'static str] = &[
    "_http_agent",
    "_http_client",
    "_http_common",
    "_http_incoming",
    "_http_outgoing",
    "_http_server",
    "_stream_duplex",
    "_stream_passthrough",
    "_stream_readable",
    "_stream_transform",
    "_stream_wrap",
    "_stream_writable",
    "_tls_common",
    "_tls_wrap",
    "assert",
    "assert/strict",
    "async_hooks",
    "buffer",
    "child_process",
    "cluster",
    "console",
    "constants",
    "crypto",
    "dgram",
    "diagnostics_channel",
    "dns",
    "dns/promises",
    "domain",
    "events",
    "fs",
    "fs/promises",
    "http",
    "http2",
    "https",
    "inspector",
    "module",
    "net",
    "os",
    "path",
    "path/posix",
    "path/win32",
    "perf_hooks",
    "process",
    "punycode",
    "querystring",
    "readline",
    "repl",
    "stream",
    "stream/promises",
    "string_decoder",
    "sys",
    "timers",
    "timers/promises",
    "tls",
    "trace_events",
    "tty",
    "url",
    "util",
    "util/types",
    "v8",
    "vm",
    "worker_threads",
    "zlib",
];

/// Determine if a package is a core package.
pub fn is_core_package(s: &str) -> bool {
    NODE_LATEST_STABLE.contains(&s)
}

/// Determine if a specifier looks like a package local path.
///
/// A local path is one that uses either a relative or absolute
/// file system path.
pub fn is_local_specifier(s: &str) -> bool {
    s.starts_with("./") || s.starts_with("../") || s.starts_with("/")
}

/// Analyze the ESM and CommonJS dependencies for a module.
pub struct ModuleDependencyAnalysis<'a> {
    file_name: &'a FileName,
    //source_map: &'a SourceMap,
    //module: &'a Module,
    //comments: &'a SingleThreadedComments,
    dependencies: Vec<DependencyDescriptor>,
}

impl<'a> ModuleDependencyAnalysis<'a> {
    /// Create a module dependency analyzer.
    pub fn new(
        file_name: &'a FileName,
        source_map: &'a SourceMap,
        module: &'a Module,
        comments: &'a SingleThreadedComments,
    ) -> Self {
        Self {
            file_name,
            //source_map,
            //module,
            //comments,
            dependencies: analyze_dependencies(module, source_map, comments),
        }
    }

    /// Get a list of builtin packages.
    pub fn builtins(&self) -> Vec<&DependencyDescriptor> {
        self.dependencies
            .iter()
            .filter_map(|dep| {
                if is_core_package(dep.specifier.as_ref()) {
                    Some(dep)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get a list of packages that are not builtins.
    pub fn packages(&self) -> Vec<&DependencyDescriptor> {
        self.dependencies
            .iter()
            .filter_map(|dep| {
                if !is_core_package(dep.specifier.as_ref())
                    && !is_local_specifier(dep.specifier.as_ref())
                {
                    Some(dep)
                } else {
                    None
                }
            })
            .collect()
    }
}
