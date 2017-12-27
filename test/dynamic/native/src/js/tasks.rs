use std;

use neon::vm::{Call, JsResult};
use neon::scope::{Scope};
use neon::js::{JsUndefined, JsNumber, JsFunction, JsString};
use neon::js::error::{Kind, JsError};
use neon::task::Task;

struct SuccessTask;

impl Task for SuccessTask {
    type Output = i32;
    type Error = String;
    type JsEvent = JsNumber;

    fn perform(&self) -> Result<Self::Output, Self::Error> {
        Ok(17)
    }

    fn complete<'a, T: Scope<'a>>(self, scope: &'a mut T, result: Result<Self::Output, Self::Error>) -> JsResult<Self::JsEvent> {
        Ok(JsNumber::new(scope, result.unwrap() as f64))
    }
}

pub fn perform_async_task(call: Call) -> JsResult<JsUndefined> {
    let f = call.arguments.require(call.scope, 0)?.check::<JsFunction>()?;
    SuccessTask.schedule(f);
    Ok(JsUndefined::new())
}

struct FailureTask;

impl Task for FailureTask {
    type Output = i32;
    type Error = String;
    type JsEvent = JsNumber;

    fn perform(&self) -> Result<Self::Output, Self::Error> {
        Err(format!("I am a failing task"))
    }

    fn complete<'a, T: Scope<'a>>(self, _: &'a mut T, result: Result<Self::Output, Self::Error>) -> JsResult<Self::JsEvent> {
        JsError::throw(Kind::Error, &result.unwrap_err())
    }
}

pub fn perform_failing_task(call: Call) -> JsResult<JsUndefined> {
    let f = call.arguments.require(call.scope, 0)?.check::<JsFunction>()?;
    FailureTask.schedule(f);
    Ok(JsUndefined::new())
}

struct OwnedTask(String);

impl Task for OwnedTask {
    type Output = String;
    type Error = String;
    type JsEvent = JsString;

    fn perform(&self) -> Result<Self::Output, Self::Error> {
        Ok(format!("Hello, {}!", self.0))
    }

    fn complete<'a, T: Scope<'a>>(self, scope: &'a mut T, result: Result<Self::Output, Self::Error>) -> JsResult<Self::JsEvent> {
        Ok(JsString::new(scope, &result.unwrap()).unwrap())
    }
}

pub fn perform_owned_task(call: Call) -> JsResult<JsUndefined> {
    let s = call.arguments.require(call.scope, 0)?.check::<JsString>()?;
    let f = call.arguments.require(call.scope, 1)?.check::<JsFunction>()?;

    OwnedTask(s.value()).schedule(f);

    Ok(JsUndefined::new())
}

struct BorrowedTask<'b>(&'b str);

impl<'b> Task for BorrowedTask<'b> {
    type Output = String;
    type Error = String;
    type JsEvent = JsString;

    fn perform(&self) -> Result<Self::Output, Self::Error> {
        Ok(format!("Hello, {}!", self.0))
    }

    fn complete<'a, T: Scope<'a>>(self, scope: &'a mut T, result: Result<Self::Output, Self::Error>) -> JsResult<Self::JsEvent> {
        Ok(JsString::new(scope, &result.unwrap()).unwrap())
    }
}

pub fn perform_borrowed_task(call: Call) -> JsResult<JsUndefined> {
    let s = call.arguments.require(call.scope, 0)?.check::<JsString>()?;
    let f = call.arguments.require(call.scope, 1)?.check::<JsFunction>()?;

    BorrowedTask(&s.value()).schedule(f);

    Ok(JsUndefined::new())
}

pub fn perform_borrowed_task_static(call: Call) -> JsResult<JsUndefined> {
    let f = call.arguments.require(call.scope, 0)?.check::<JsFunction>()?;

    BorrowedTask("World").schedule(f);

    Ok(JsUndefined::new())
}

pub fn perform_borrowed_task_static_short(call: Call) -> JsResult<JsUndefined> {
    let f = call.arguments.require(call.scope, 0)?.check::<JsFunction>()?;

    BorrowedTask("World".to_owned().as_str()).schedule(f);

    Ok(JsUndefined::new())
}

pub fn perform_borrowed_task_forgot(call: Call) -> JsResult<JsUndefined> {
    let s = call.arguments.require(call.scope, 0)?.check::<JsString>()?;
    let f = call.arguments.require(call.scope, 1)?.check::<JsFunction>()?;
    let v = s.value();

    BorrowedTask(&v).schedule(f);
    std::mem::forget(v);

    Ok(JsUndefined::new())
}
