use neon::prelude::*;
use neon::result::JsContextResult;
use neon::object::This;

fn add1(cx: FunctionContext) -> JsResult<JsNumber> {
    let (mut cx, x) = cx.argument::<JsNumber>(0)?;
    let x = x.value();
    Ok(cx.number(x + 1.0))
}

pub fn return_js_function(mut cx: FunctionContext) -> JsResult<JsFunction> {
    JsFunction::new(&mut cx, add1)
}

pub fn call_js_function(cx: FunctionContext) -> JsResult<JsNumber> {
    let (mut cx, f) = cx.argument::<JsFunction>(0)?;
    let args: Vec<Handle<JsNumber>> = vec![cx.number(16.0)];
    let null = cx.null();
    let (cx, res) = f.call(cx, null, args)?;

    res.downcast::<JsNumber>()
        .or_throw(cx)
        .map(|(_, res)| res)
}

pub fn construct_js_function(cx: FunctionContext) -> JsResult<JsNumber> {
    let (mut cx, f) = cx.argument::<JsFunction>(0)?;
    let zero = cx.number(0.0);
    let (mut cx, o) = f.construct(cx, vec![zero])?;
    let (cx, get_utc_full_year_method) = o.get(&mut cx, "getUTCFullYear")?.downcast::<JsFunction>().or_throw(cx)?;
    let args: Vec<Handle<JsValue>> = vec![];
    let (cx, res) = get_utc_full_year_method.call(cx, o.upcast::<JsValue>(), args)?;
    res.downcast::<JsNumber>().or_throw(cx).map(|(_, res)| res)
}

trait CheckArgument<'a, C> {
    fn check_argument<V: Value>(self, i: i32) -> JsContextResult<'a, C, V>;
}

impl<'a, T: This> CheckArgument<'a, CallContext<'a, T>> for CallContext<'a, T> {
    fn check_argument<V: Value>(self, i: i32) -> JsContextResult<'a, CallContext<'a, T>, V> {
        self.argument::<V>(i)
    }
}

pub fn check_string_and_number(cx: FunctionContext) -> JsResult<JsUndefined> {
    let (cx, _) = cx.check_argument::<JsString>(0)?;
    let (mut cx, _) = cx.check_argument::<JsNumber>(1)?;
    Ok(cx.undefined())
}

pub fn panic(_: FunctionContext) -> JsResult<JsUndefined> {
    panic!("zomg")
}

pub fn panic_after_throw(cx: FunctionContext) -> JsResult<JsUndefined> {
    cx.throw_range_error::<_, ()>("entering throw state with a RangeError").unwrap_err();
    panic!("this should override the RangeError")
}

pub fn num_arguments(mut cx: FunctionContext) -> JsResult<JsNumber> {
    let n = cx.len();
    Ok(cx.number(n))
}

pub fn return_this(mut cx: FunctionContext) -> JsResult<JsValue> {
    Ok(cx.this().upcast())
}

pub fn require_object_this(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let this = cx.this();
    let (mut cx, this) = this.downcast::<JsObject>().or_throw(cx)?;
    let t = cx.boolean(true);
    this.set(&mut cx, "modified", t)?;
    Ok(cx.undefined())
}

pub fn is_argument_zero_some(mut cx: FunctionContext) -> JsResult<JsBoolean> {
    let b = cx.argument_opt(0).is_some();
    Ok(cx.boolean(b))
}

pub fn require_argument_zero_string(cx: FunctionContext) -> JsResult<JsString> {
    let (_, s) = cx.argument(0)?;
    Ok(s)
}

pub fn execute_scoped(mut cx: FunctionContext) -> JsResult<JsNumber> {
    let mut i = 0;
    for _ in 1..100 {
        cx.execute_scoped(|mut cx| {
            let n = cx.number(1);
            i += n.value() as i32;
        });
    }
    Ok(cx.number(i))
}

pub fn compute_scoped(mut cx: FunctionContext) -> JsResult<JsNumber> {
    let mut i = cx.number(0);
    for _ in 1..100 {
        i = cx.compute_scoped(|mut cx| {
            let n = cx.number(1);
            Ok(cx.number((i.value() as i32) + (n.value() as i32)))
        })?;
    }
    Ok(i)
}
