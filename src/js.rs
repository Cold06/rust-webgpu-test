use rquickjs::{CatchResultExt, Context, Function, Object, Runtime, Value};
use skia_safe::wrapper::NativeTransmutableWrapper;

pub struct VM {
    runtime: Runtime,
    ctx: Context,
}

fn print(s: String) {
    println!("{s}");
}

impl VM {
    pub fn new() -> Self {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();

        ctx.with(|ctx| -> Result<(), ()> {
            let global = ctx.globals();
            global
                .set(
                    "__print",
                    Function::new(ctx.clone(), print)
                        .unwrap()
                        .with_name("__print")
                        .unwrap(),
                )
                .unwrap();
            ctx.eval::<(), _>(
                r#"
globalThis.console = {
  log(...v) {
    globalThis.__print(`${v.join(" ")}`)
  }
}
"#,
            )
            .unwrap();

            Ok(())
        })
        .unwrap();

        Self {
            ctx: ctx,
            runtime: rt,
        }
    }

    pub fn eval(&self, script: &str) {
        self.ctx.with(|ctx| {
            let global = ctx.globals();
            let console: Object = global.get("console").unwrap();
            let js_log: Function = console.get("log").unwrap();

            ctx.eval::<Value, _>(script.as_bytes())
                .and_then(|ret| js_log.call::<(Value<'_>,), ()>((ret,)))
                .catch(&ctx)
                .unwrap_or_else(|err| println!("{err}"));
        });
    }
}
