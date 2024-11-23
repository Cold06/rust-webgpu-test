use crate::canvas::Canvas;
use rquickjs::{CatchResultExt, Context, Function, Object, Runtime, Value};
use std::cell::RefCell;
use std::rc::Rc;

pub struct VM {
    runtime: Runtime,
    ctx: Context,
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
                    Function::new(ctx.clone(), |s: String| {
                        println!("{s}");
                    })
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

    pub fn eval_with_canvas(&mut self, code: String, canvas: Rc<RefCell<Canvas>>) {
        self.ctx
            .with(|ctx| -> Result<(), ()> {
                let global = ctx.globals();

                let canvas_0 = canvas.clone();
                let canvas_1 = canvas.clone();
                let canvas_2 = canvas.clone();
                let canvas_3 = canvas.clone();
                let canvas_4 = canvas.clone();
                let canvas_5 = canvas.clone();
                let canvas_6 = canvas.clone();
                let canvas_7 = canvas.clone();
                let canvas_8 = canvas.clone();
                let canvas_9 = canvas.clone();
                let canvas_10 = canvas.clone();
                let canvas_11 = canvas.clone();
                let canvas_12 = canvas.clone();
                let canvas_13 = canvas.clone();
                let canvas_14 = canvas.clone();
                let canvas_15 = canvas.clone();
                let canvas_16 = canvas.clone();
                let canvas_17 = canvas.clone();
                let canvas_18 = canvas.clone();




                global
                    .set(
                        "__js_set_fill_style",
                        Function::new(ctx.clone(), move |rgb_color: String| {
                            canvas_0.borrow_mut().js_set_fill_style(rgb_color);
                        })
                        .unwrap()
                        .with_name("__js_set_fill_style")
                        .unwrap(),
                    )
                    .unwrap();
                global
                    .set(
                        "__js_set_line_width",
                        Function::new(ctx.clone(), move |line_width: f64| {
                            canvas_1.borrow_mut().js_set_line_width(line_width);
                        })
                        .unwrap()
                        .with_name("__js_set_line_width")
                        .unwrap(),
                    )
                    .unwrap();
                global
                    .set(
                        "__js_set_stroke_style",
                        Function::new(ctx.clone(), move |stroke_style: String| {
                            canvas_2.borrow_mut().js_set_stroke_style(stroke_style);
                        })
                        .unwrap()
                        .with_name("__js_set_stroke_style")
                        .unwrap(),
                    )
                    .unwrap();
                global
                    .set(
                        "__js_set_line_cap",
                        Function::new(ctx.clone(), move |line_cap: String| {
                            canvas_3.borrow_mut().js_set_line_cap(line_cap);
                        })
                        .unwrap()
                        .with_name("__js_set_line_cap")
                        .unwrap(),
                    )
                    .unwrap();
                global
                    .set(
                        "__js_set_line_join",
                        Function::new(ctx.clone(), move |line_join: String| {
                            canvas_4.borrow_mut().js_set_line_join(line_join);
                        })
                        .unwrap()
                        .with_name("__js_set_line_join")
                        .unwrap(),
                    )
                    .unwrap();
                global
                    .set(
                        "__js_set_miter_limit",
                        Function::new(ctx.clone(), move |miter_limit: f64| {
                            canvas_5.borrow_mut().js_set_miter_limit(miter_limit);
                        })
                        .unwrap()
                        .with_name("__js_set_miter_limit")
                        .unwrap(),
                    )
                    .unwrap();
                global
                    .set(
                        "__js_call_save",
                        Function::new(ctx.clone(), move || {
                            canvas_6.borrow_mut().js_call_save();
                        })
                        .unwrap()
                        .with_name("__js_call_save")
                        .unwrap(),
                    )
                    .unwrap();
                global
                    .set(
                        "__js_call_scale",
                        Function::new(ctx.clone(), move |x: f64, y: f64| {
                            canvas_7.borrow_mut().js_call_scale(x, y);
                        })
                        .unwrap()
                        .with_name("__js_call_scale")
                        .unwrap(),
                    )
                    .unwrap();
                global
                    .set(
                        "__js_call_begin_path",
                        Function::new(ctx.clone(), move || {
                            canvas_8.borrow_mut().js_call_begin_path();
                        })
                        .unwrap()
                        .with_name("__js_call_begin_path")
                        .unwrap(),
                    )
                    .unwrap();
                global
                    .set(
                        "__js_call_rect",
                        Function::new(
                            ctx.clone(),
                            move |x: f64, y: f64, width: f64, height: f64| {
                                canvas_9.borrow_mut().js_call_rect(x, y, width, height);
                            },
                        )
                        .unwrap()
                        .with_name("__js_call_rect")
                        .unwrap(),
                    )
                    .unwrap();
                global
                    .set(
                        "__js_call_fill",
                        Function::new(ctx.clone(), move || {
                            canvas_10.borrow_mut().js_call_fill();
                        })
                        .unwrap()
                        .with_name("__js_call_fill")
                        .unwrap(),
                    )
                    .unwrap();
                global
                    .set(
                        "__js_call_move_to",
                        Function::new(ctx.clone(), move |x: f64, y: f64| {
                            canvas_11.borrow_mut().js_call_move_to(x, y);
                        })
                        .unwrap()
                        .with_name("__js_call_move_to")
                        .unwrap(),
                    )
                    .unwrap();
                global
                    .set(
                        "__js_call_line_to",
                        Function::new(ctx.clone(), move |x: f64, y: f64| {
                            canvas_12.borrow_mut().js_call_line_to(x, y);
                        })
                        .unwrap()
                        .with_name("__js_call_line_to")
                        .unwrap(),
                    )
                    .unwrap();
                global
                    .set(
                        "__js_call_bezier_curve_to",
                        Function::new(
                            ctx.clone(),
                            move |p1x: f64, p1y: f64, p2x: f64, p2y: f64, px: f64, py: f64| {
                                canvas_13
                                    .borrow_mut()
                                    .js_call_bezier_curve_to(p1x, p1y, p2x, p2y, px, py);
                            },
                        )
                        .unwrap()
                        .with_name("__js_call_bezier_curve_to")
                        .unwrap(),
                    )
                    .unwrap();
                global
                    .set(
                        "__js_call_close_path",
                        Function::new(ctx.clone(), move || {
                            canvas_14.borrow_mut().js_call_close_path();
                        })
                        .unwrap()
                        .with_name("__js_call_close_path")
                        .unwrap(),
                    )
                    .unwrap();
                global
                    .set(
                        "__js_call_transform",
                        Function::new(
                            ctx.clone(),
                            move |a: f64, b: f64, c: f64, d: f64, e: f64, f: f64| {
                                canvas_15.borrow_mut().js_call_transform(a, b, c, d, e, f);
                            },
                        )
                        .unwrap()
                        .with_name("__js_call_transform")
                        .unwrap(),
                    )
                    .unwrap();
                global
                    .set(
                        "__js_call_stroke",
                        Function::new(ctx.clone(), move || {
                            canvas_16.borrow_mut().js_call_stroke();
                        })
                        .unwrap()
                        .with_name("__js_call_stroke")
                        .unwrap(),
                    )
                    .unwrap();
                global
                    .set(
                        "__js_call_restore",
                        Function::new(ctx.clone(), move || {
                            canvas_17.borrow_mut().js_call_restore();
                        })
                        .unwrap()
                        .with_name("__js_call_restore")
                        .unwrap(),
                    )
                    .unwrap();
                global
                    .set(
                        "__js_call_arc",
                        Function::new(ctx.clone(), move |x: f64, y: f64, radius: f64, start_angle: f64, end_angle: f64, counterclockwise: bool| {
                            canvas_18.borrow_mut().js_call_arc(x, y, radius, start_angle, end_angle, counterclockwise);
                        })
                            .unwrap()
                            .with_name("__js_call_arc")
                            .unwrap(),
                    )
                    .unwrap();



                ctx.eval::<(), _>(
                    r#"
globalThis.ctx = {
  set globalAlpha(value) { /**/ },
  set fillStyle(value) { globalThis.__js_set_fill_style(value); },
  set lineWidth(value) { globalThis.__js_set_line_width(value); },
  set strokeStyle(value) { globalThis.__js_set_stroke_style(value); },
  set lineCap(value) { globalThis.__js_set_line_cap(value); },
  set lineJoin(value) { globalThis.__js_set_line_join(value); },
  set miterLimit(value) { globalThis.__js_set_miter_limit(value); },
  save(...args) { globalThis.__js_call_save(...args); },
  scale(...args) { globalThis.__js_call_scale(...args); },
  beginPath(...args) { globalThis.__js_call_begin_path(...args); },
  rect(...args) { globalThis.__js_call_rect(...args); },
  fill(...args) { globalThis.__js_call_fill(...args); },
  moveTo(...args) { globalThis.__js_call_move_to(...args); },
  lineTo(...args) { globalThis.__js_call_line_to(...args); },
  bezierCurveTo(...args) { globalThis.__js_call_bezier_curve_to(...args); },
  closePath(...args) { globalThis.__js_call_close_path(...args); },
  transform(...args) { globalThis.__js_call_transform(...args); },
  stroke(...args) { globalThis.__js_call_stroke(...args); },
  restore(...args) { globalThis.__js_call_restore(...args); },
  arc(...args) { globalThis.__js_call_arc(...args); },
};
"#,
                )
                .unwrap();


                {
                    canvas.borrow_mut().save();
                }

                match ctx.eval::<(), _>(code) {
                    Ok(_) => {}
                    Err(err) => {
                        println!("{err}");
                    }
                }

                {
                    canvas.borrow_mut().js_call_restore();
                }

                Ok(())
            })
            .unwrap();
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
