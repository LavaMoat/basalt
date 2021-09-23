#[macro_use]
extern crate napi_derive;

use napi::{CallContext, JsObject, JsString, JsUndefined, Result};
use std::ffi::OsString;

#[module_exports]
fn init(mut exports: JsObject) -> Result<()> {
    exports.create_named_method("run", run)?;
    Ok(())
}

#[js_function(1)]
fn run(ctx: CallContext) -> Result<JsUndefined> {
    let mut argv: Vec<OsString> = Vec::new();
    let args = ctx.get::<JsObject>(0)?;
    if args.is_array()? {
        for i in 0..args.get_array_length_unchecked()? {
            let arg =
                args.get_element::<JsString>(i)?.into_utf8()?.into_owned()?;
            argv.push(OsString::from(arg));
        }

        if let Err(e) = basalt::cli::run::<OsString>(Some(argv)) {
            panic!("{}", e);
        }
    } else {
        panic!("run(): argv must be an array of strings");
    }

    ctx.env.get_undefined()
}
