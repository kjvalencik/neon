//! Types and traits for working with JavaScript exceptions.

use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};
use handle::Handle;
use types::Value;
use context::Context;

/// An error sentinel type used by `NeonResult` (and `JsResult`) to indicate that the JS engine has entered into a throwing state.
#[derive(Debug)]
pub struct Throw;

impl Display for Throw {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        fmt.write_str("JavaScript Error")
    }
}

impl Error for Throw {
    fn description(&self) -> &str {
        "javascript error"
    }
}

/// The result of a computation that might send the JS engine into a throwing state.
pub type NeonResult<T> = Result<T, Throw>;
pub type NeonContextResult<C, T> = NeonResult<(C, T)>;

/// The result of a computation that produces a JavaScript value and might send the JS engine into a throwing state.
pub type JsResult<'b, T> = NeonResult<Handle<'b, T>>;
pub type JsContextResult<'b, C, T> = NeonContextResult<C, Handle<'b, T>>;

/// An extension trait for `Result` values that can be converted into `JsResult` values by throwing a JavaScript
/// exception in the error case.
pub trait JsResultExt<'a, V: Value> {
    fn or_throw<'b, C: Context<'b>>(self, cx: C) -> JsContextResult<'a, C, V>;
}
