//! Detect built in packages for node declared as ESM imports
//! or CommonJS require expressions.
//!
//! Run `node -p "require('module').builtinModules"` to generate the
//! list of built in packages for a version of node.
//!

use swc_common::{comments::SingleThreadedComments, SourceMap};
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

/// Analyze the dependencies for a module and return a list
/// of the dependencies that are builtin packages.
pub fn analyze(
    module: &Module,
    source_map: &SourceMap,
) -> Vec<DependencyDescriptor> {
    let comments: SingleThreadedComments = Default::default();
    analyze_dependencies(module, source_map, &comments)
        .into_iter()
        .filter_map(|dep| {
            if is_core_package(dep.specifier.as_ref()) {
                Some(dep)
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
}
