//! Types and traits representing JavaScript error values.

use std::panic::{UnwindSafe, catch_unwind};

use neon_runtime;
use neon_runtime::raw;

use context::Context;
use result::{NeonContextResult, NeonResult, Throw};
use types::{Value, Object, Handle, Managed, build};
use types::internal::ValueInternal;
use types::utf8::Utf8;

/// A JS `Error` object.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct JsError(raw::Local);

impl Managed for JsError {
    fn to_raw(self) -> raw::Local { self.0 }

    fn from_raw(h: raw::Local) -> Self { JsError(h) }
}

impl ValueInternal for JsError {
    fn name() -> String { "Error".to_string() }

    fn is_typeof<Other: Value>(other: Other) -> bool {
        unsafe { neon_runtime::tag::is_error(other.to_raw()) }
    }
}

impl Value for JsError { }

impl Object for JsError { }

impl JsError {
    /// Creates a direct instance of the [`Error`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Error) class.
    pub fn error<'a, C: Context<'a>, S: AsRef<str>>(mut cx: C, msg: S) -> NeonContextResult<C, Handle<'a, JsError>> {
        let msg = cx.string(msg.as_ref());
        build(|out| unsafe {
            neon_runtime::error::new_error(out, msg.to_raw());
            true
        }).map(|res| (cx, res))
    }

    /// Creates an instance of the [`TypeError`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/TypeError) class.
    pub fn type_error<'a, C: Context<'a>, S: AsRef<str>>(mut cx: C, msg: S) -> NeonContextResult<C, Handle<'a, JsError>> {
        let msg = cx.string(msg.as_ref());
        build(|out| unsafe {
            neon_runtime::error::new_type_error(out, msg.to_raw());
            true
        }).map(|res| (cx, res))
    }

    /// Creates an instance of the [`RangeError`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/RangeError) class.
    pub fn range_error<'a, C: Context<'a>, S: AsRef<str>>(mut cx: C, msg: S) -> NeonContextResult<C, Handle<'a, JsError>> {
        let msg = cx.string(msg.as_ref());
        build(|out| unsafe {
            neon_runtime::error::new_range_error(out, msg.to_raw());
            true
        }).map(|res| (cx, res))
    }
}

pub(crate) fn convert_panics<T, F: UnwindSafe + FnOnce() -> NeonResult<T>>(f: F) -> NeonResult<T> {
    match catch_unwind(|| { f() }) {
        Ok(result) => result,
        Err(panic) => {
            let msg = if let Some(string) = panic.downcast_ref::<String>() {
                format!("internal error in Neon module: {}", string)
            } else if let Some(str) = panic.downcast_ref::<&str>() {
                format!("internal error in Neon module: {}", str)
            } else {
                "internal error in Neon module".to_string()
            };
            let (data, len) = Utf8::from(&msg[..]).truncate().lower();
            unsafe {
                neon_runtime::error::throw_error_from_utf8(data, len);
                Err(Throw)
            }
        }
    }
}
