use std;
#[cfg(feature = "legacy-runtime")]
use std::any::Any;
use std::boxed::Box;
use std::cell::Cell;
use std::mem::MaybeUninit;
use std::os::raw::c_void;
#[cfg(feature = "legacy-runtime")]
use std::panic::{AssertUnwindSafe, UnwindSafe, catch_unwind, resume_unwind};
use neon_runtime;
use neon_runtime::raw;
use neon_runtime::scope::Root;
#[cfg(feature = "legacy-runtime")]
use neon_runtime::try_catch::TryCatchControl;
use types::{JsObject, JsValue, Value};
use handle::Handle;
use object::class::ClassMap;
use result::{JsResult, NeonResult};
use super::ModuleContext;

#[cfg(feature = "legacy-runtime")]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Env(raw::Isolate);

#[cfg(feature = "napi-runtime")]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Env(raw::Env);

extern "C" fn drop_class_map(map: Box<ClassMap>) {
    std::mem::drop(map);
}

impl Env {
    #[cfg(feature = "legacy-runtime")]
    pub(crate) fn to_raw(self) -> raw::Isolate {
        let Self(ptr) = self;
        ptr
    }

    #[cfg(feature = "napi-runtime")]
    pub(crate) fn to_raw(self) -> raw::Env {
        let Self(ptr) = self;
        ptr
    }

    pub(crate) fn class_map(&mut self) -> &mut ClassMap {
        let mut ptr: *mut c_void = unsafe { neon_runtime::class::get_class_map(self.to_raw()) };
        if ptr.is_null() {
            let b: Box<ClassMap> = Box::new(ClassMap::new());
            let raw = Box::into_raw(b);
            ptr = unsafe { std::mem::transmute(raw) };
            let free_map: *mut c_void = unsafe { std::mem::transmute(drop_class_map as usize) };
            unsafe {
                neon_runtime::class::set_class_map(self.to_raw(), ptr, free_map);
            }
        }
        unsafe { std::mem::transmute(ptr) }
    }

    #[cfg(feature = "napi-runtime")]
    pub(crate) fn current() -> Env {
        panic!("Context::current() will not implemented with n-api")
    }

    #[cfg(feature = "legacy-runtime")]
    pub(crate) fn current() -> Env {
        unsafe {
            std::mem::transmute(neon_runtime::call::current_isolate())
        }
    }
}

pub struct ScopeMetadata {
    env: Env,
    active: Cell<bool>
}

pub struct Scope<'a, R: Root + 'static> {
    pub metadata: ScopeMetadata,
    pub handle_scope: &'a mut R
}

impl<'a, R: Root + 'static> Scope<'a, R> {
    pub fn with<T, F: for<'b> FnOnce(Scope<'b, R>) -> T>(env: Env, f: F) -> T {
        let mut handle_scope: R = unsafe { R::allocate() };
        unsafe {
            handle_scope.enter(env.to_raw());
        }
        let result = {
            let scope = Scope {
                metadata: ScopeMetadata {
                    env,
                    active: Cell::new(true)
                },
                handle_scope: &mut handle_scope
            };
            f(scope)
        };
        unsafe {
            handle_scope.exit(env.to_raw());
        }
        result
    }
}

pub trait ContextInternal<'a>: Sized {
    fn scope_metadata(&self) -> &ScopeMetadata;

    fn env(&self) -> Env {
        self.scope_metadata().env
    }

    fn is_active(&self) -> bool {
        self.scope_metadata().active.get()
    }

    fn check_active(&self) {
        if !self.is_active() {
            panic!("execution context is inactive");
        }
    }

    fn activate(&self) { self.scope_metadata().active.set(true); }
    fn deactivate(&self) { self.scope_metadata().active.set(false); }

    #[cfg(feature = "legacy-runtime")]
    fn try_catch_internal<'b: 'a, T, F>(&mut self, f: F) -> Result<Handle<'a, T>, Handle<'a, JsValue>>
        where T: Value,
              F: UnwindSafe + FnOnce(&mut Self) -> JsResult<'b, T>
    {
        // A closure does not have a guaranteed layout, so we need to box it in order to pass
        // a pointer to it across the boundary into C++.
        let rust_thunk = Box::into_raw(Box::new(f));

        let mut local: MaybeUninit<raw::Local> = MaybeUninit::zeroed();
        let mut unwind_value: MaybeUninit<*mut c_void> = MaybeUninit::zeroed();

        let ctrl = unsafe {
            neon_runtime::try_catch::with(try_catch_glue::<Self, T, F>,
                                          rust_thunk as *mut c_void,
                                          (self as *mut Self) as *mut c_void,
                                          local.as_mut_ptr(),
                                          unwind_value.as_mut_ptr())
        };

        match ctrl {
            TryCatchControl::Panicked => {
                let unwind_value: Box<dyn Any + Send> = *unsafe {
                    Box::from_raw(unwind_value.assume_init() as *mut Box<dyn Any + Send>)
                };
                resume_unwind(unwind_value);
            }
            TryCatchControl::Returned => {
                let local = unsafe { local.assume_init() };
                Ok(Handle::new_internal(T::from_raw(local)))
            }
            TryCatchControl::Threw => {
                let local = unsafe { local.assume_init() };
                Err(JsValue::new_internal(local))
            }
            TryCatchControl::UnexpectedErr => {
                panic!("try_catch: unexpected Err(Throw) when VM is not in a throwing state");
            }
        }
    }

    #[cfg(feature = "napi-runtime")]
    fn try_catch_internal<'b: 'a, T, F>(&mut self, f: F) -> Result<Handle<'a, T>, Handle<'a, JsValue>>
        where T: Value,
              F: FnOnce(&mut Self) -> JsResult<'b, T>
    {
        let result = f(self);
        let mut local: MaybeUninit<raw::Local> = MaybeUninit::zeroed();
        unsafe {
            if neon_runtime::error::catch_error(self.env().to_raw(), local.as_mut_ptr()) {
                Err(JsValue::new_internal(local.assume_init()))
            } else if let Ok(result) = result {
                Ok(result)
            } else {
                panic!("try_catch: unexpected Err(Throw) when VM is not in a throwing state");
            }
        }
    }

    #[cfg(feature = "napi-runtime")]
    fn global_queue(&mut self) -> crate::sync::EventQueue {
        let mut data = std::mem::MaybeUninit::uninit();
        let status = unsafe {
            napi::napi_get_instance_data(
                self.env().to_raw(),
                data.as_mut_ptr(),
            )
        };

        // FIXME: This should return an error instead of a panic
        assert_eq!(status, neon_runtime::sys::napi_status::napi_ok);

        let data: Box<ModuleInstanceData> = unsafe {
            Box::from_raw(data.assume_init() as *mut _)
        };

        let queue = data.queue.clone();

        Box::leak(data);
    
        queue
    }
}

#[cfg(feature = "legacy-runtime")]
extern "C" fn try_catch_glue<'a, 'b: 'a, C, T, F>(rust_thunk: *mut c_void,
                                                  cx: *mut c_void,
                                                  returned: *mut raw::Local,
                                                  unwind_value: *mut *mut c_void) -> TryCatchControl
    where C: ContextInternal<'a>,
          T: Value,
          F: UnwindSafe + FnOnce(&mut C) -> JsResult<'b, T>
{
    let f: F = *unsafe { Box::from_raw(rust_thunk as *mut F) };
    let cx: &mut C = unsafe { std::mem::transmute(cx) };

    // The mutable reference to the context is a fiction of the Neon library,
    // since it doesn't actually contain any data in the Rust memory space,
    // just a link to the JS VM. So we don't need to do any kind of poisoning
    // of the context when a panic occurs. So we suppress the Rust compiler
    // errors from using the mutable reference across an unwind boundary.
    match catch_unwind(AssertUnwindSafe(|| f(cx))) {
        // No Rust panic, no JS exception.
        Ok(Ok(result)) => unsafe {
            *returned = result.to_raw();
            TryCatchControl::Returned
        }
        // No Rust panic, caught a JS exception.
        Ok(Err(_)) => {
            TryCatchControl::Threw
        }
        // Rust panicked.
        Err(err) => unsafe {
            // A panic value has an undefined layout, so wrap it in an extra box.
            let boxed = Box::new(err);
            *unwind_value = Box::into_raw(boxed) as *mut c_void;
            TryCatchControl::Panicked
        }
    }
}

#[cfg(feature = "napi-runtime")]
struct ModuleInstanceData {
    queue: crate::sync::EventQueue,
}

#[cfg(feature = "napi-runtime")]
impl ModuleInstanceData {
    fn new(cx: &mut ModuleContext) -> Self {
        ModuleInstanceData {
            queue: crate::sync::EventQueue::new(cx)
                .expect("Failed to module global queue")
                .unref(cx)
                .expect("Failed to unref global queue"),
        }
    }
}

#[cfg(feature = "napi-runtime")]
extern "C" fn drop_module_instance_data(
    _env: neon_runtime::sys::napi_env,
    finalize_data: *mut std::os::raw::c_void,
    _finalize_hint: *mut std::os::raw::c_void,
) {
    unsafe {
        Box::<ModuleInstanceData>::from_raw(finalize_data as *mut _);
    }
}

// TODO: These should be moved to nodejs-sys
#[cfg(feature = "napi-runtime")]
mod napi {
    use neon_runtime::sys::*;

    extern "C" {
            pub fn napi_set_instance_data(
                env: napi_env,
                data: *mut ::std::os::raw::c_void,
                finalize_cb: napi_finalize,
                finalize_hint: *mut ::std::os::raw::c_void,
            ) -> napi_status;

            pub fn napi_get_instance_data(
                env: napi_env,
                data: *mut *mut ::std::os::raw::c_void,
            ) -> napi_status;
    }
}

#[cfg(feature = "legacy-runtime")]
pub fn initialize_module(exports: Handle<JsObject>, init: fn(ModuleContext) -> NeonResult<()>) {
    let env = Env::current();

    ModuleContext::with(env, exports, |cx| {
        let _ = init(cx);
    });
}

#[cfg(feature = "napi-runtime")]
pub fn initialize_module(env: raw::Env, exports: Handle<JsObject>, init: fn(ModuleContext) -> NeonResult<()>) {
    // crate::sync::initialize_unref_callback(env);
    // crate::sync::initialize_threadsafe_closure_callback(env);

    ModuleContext::with(Env(env), exports, |mut cx| {
        let data = Box::new(ModuleInstanceData::new(&mut cx));
        let status = unsafe {
            napi::napi_set_instance_data(
                cx.env().to_raw(),
                Box::into_raw(data) as *mut _,
                Some(drop_module_instance_data),
                std::ptr::null_mut(),
            )
        };

        if status == neon_runtime::sys::napi_status::napi_ok {
            let _ = init(cx);
        }
    });
}
