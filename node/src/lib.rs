#[macro_use]
extern crate napi_derive;

use napi::{CallContext, JsUndefined, JsString, JsObject, Result};

#[module_exports]
fn init(mut exports: JsObject) -> Result<()> {
  exports.create_named_method("cli", cli)?;
  Ok(())
}

#[js_function(1)]
fn cli(ctx: CallContext) -> Result<JsUndefined> {
    println!("Running native function");

    //let args: Vec<Result<JsString>> = ctx.get_all()
        //.into_iter()
        //.map(|unknown| unknown.coerce_to_string())
        //.collect();

    ctx.env.get_undefined()
}
