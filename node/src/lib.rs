#[macro_use]
extern crate napi_derive;

use napi::{CallContext, JsUndefined, JsUnknown, JsObject, Result};

#[module_exports]
fn init(mut exports: JsObject) -> Result<()> {
  exports.create_named_method("cli", cli)?;
  Ok(())
}

#[js_function(1)]
fn cli(ctx: CallContext) -> Result<JsUndefined> {
    let arg = ctx.get::<JsUnknown>(0)?;
    let argv: Vec<String> = ctx.env.from_js_value(arg)?;
    println!("{:#?}", argv);
    ctx.env.get_undefined()
}
