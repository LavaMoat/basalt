//! Detect built in packages for node declared as ESM imports
//! or CommonJS require expressions.
//!
//! Run `node -p "require('module').builtinModules"` to generate the
//! list of built in packages for a version of node.
//!

use swc_common::{comments::SingleThreadedComments, SourceMap};
use swc_ecma_ast::Module;
use swc_ecma_dep_graph::{analyze_dependencies, DependencyDescriptor};

/// List of built in packages for node@12.
pub const NODE_12: &'static [&'static str] = &[
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
    "async_hooks",
    "buffer",
    "child_process",
    "cluster",
    "console",
    "constants",
    "crypto",
    "dgram",
    "dns",
    "domain",
    "events",
    "fs",
    "http",
    "http2",
    "https",
    "inspector",
    "module",
    "net",
    "os",
    "path",
    "perf_hooks",
    "process",
    "punycode",
    "querystring",
    "readline",
    "repl",
    "stream",
    "string_decoder",
    "sys",
    "timers",
    "tls",
    "trace_events",
    "tty",
    "url",
    "util",
    "v8",
    "vm",
    "worker_threads",
    "zlib",
];

/// List of built in packages for node@14.
pub const NODE_14: &'static [&'static str] = &[
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
    "perf_hooks",
    "process",
    "punycode",
    "querystring",
    "readline",
    "repl",
    "stream",
    "string_decoder",
    "sys",
    "timers",
    "tls",
    "trace_events",
    "tty",
    "url",
    "util",
    "v8",
    "vm",
    "worker_threads",
    "zlib",
];

/// List of built in packages for node@16.
pub const NODE_16: &'static [&'static str] = &[
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

/// Enumeration of built in lists per node version.
#[derive(Copy, Clone)]
pub enum NodeVersion {
    /// Version for node@12 series.
    Node12,
    /// Version for node@14 series.
    Node14,
    /// Version for node@16 series.
    Node16,
}

impl Default for NodeVersion {
    fn default() -> Self {
        NodeVersion::Node16
    }
}

/// Determine if a package is a core package (node@16).
pub fn is_core_package(s: &str, version: NodeVersion) -> bool {
    match version {
        NodeVersion::Node12 => NODE_12.contains(&s),
        NodeVersion::Node14 => NODE_14.contains(&s),
        NodeVersion::Node16 => NODE_16.contains(&s),
    }
}

/// Analyze the dependencies for a module and return a list
/// of the dependencies that are builtin packages.
pub fn analyze(
    module: &Module,
    source_map: &SourceMap,
    version: NodeVersion,
) -> Vec<DependencyDescriptor> {
    let comments: SingleThreadedComments = Default::default();
    analyze_dependencies(module, source_map, &comments)
        .into_iter()
        .filter_map(|dep| {
            if is_core_package(dep.specifier.as_ref(), version) {
                Some(dep)
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
}
