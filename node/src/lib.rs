#[macro_use]
extern crate napi_derive;

use std::ffi::OsString;
use napi::{CallContext, JsUndefined, JsUnknown, JsObject, Result};

#[module_exports]
fn init(mut exports: JsObject) -> Result<()> {
  exports.create_named_method("run", run)?;
  Ok(())
}

#[js_function(1)]
fn run(ctx: CallContext) -> Result<JsUndefined> {
    let arg = ctx.get::<JsUnknown>(0)?;
    let argv: Vec<String> = ctx.env.from_js_value(arg)?;
    let argv: Vec<OsString> = argv.into_iter().map(|s| OsString::from(s)).collect();
    if let Err(e) = basalt::cli::run::<OsString>(Some(argv)) {
        panic!("{}", e);
    }
    ctx.env.get_undefined()
}
