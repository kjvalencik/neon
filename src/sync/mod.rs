use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::sync::Arc;

use neon_runtime::sys as napi;

use crate::context::{Context, TaskContext};
use crate::context::internal::ContextInternal;
use crate::handle::{Handle, Managed};
use crate::result::{JsResult, NeonResult};
use crate::types::Value;

extern "C" fn threadsafe_closure_callback(
    env: napi::napi_env,
    _js_callback: napi::napi_value,
    _context: *mut ::std::os::raw::c_void,
    data: *mut ::std::os::raw::c_void,
) {
    // If the event loop has been terminated, these may be null
    if env == std::ptr::null_mut() || data == std::ptr::null_mut() {
        eprintln!("This is surprising");
        return;
    }

    let data: Box<Box<dyn for<'a> FnOnce(TaskContext<'a>)>> = unsafe {
        Box::from_raw(data as *mut _)
    };

    TaskContext::with(env, data);
}

#[derive(Debug)]
struct EventQueueInternal {
    func: napi::napi_threadsafe_function,
}

unsafe impl Send for EventQueueInternal {}
unsafe impl Sync for EventQueueInternal {}

impl EventQueueInternal {
    fn new<'a, C: Context<'a>>(cx: &mut C) -> NeonResult<Self> {
        let name = cx.string("NEON_THREADSAFE_CLOSURE_CALLBACK");

        let mut result = MaybeUninit::uninit();
        let status = unsafe {
            napi::napi_create_threadsafe_function(
                cx.env().to_raw(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                name.to_raw(),
                0,
                1,
                std::ptr::null_mut(),
                None,
                std::ptr::null_mut(),
                Some(threadsafe_closure_callback),
                result.as_mut_ptr(),
            )
        };

        if status != napi::napi_status::napi_ok {
            return cx.throw_error(
                "FIXME: Something bad happened creating reference",
            );
        }

        Ok(EventQueueInternal {
            func: unsafe { result.assume_init() },
        })
    }

    fn send(&self, f: impl FnOnce(TaskContext)) -> Result<(), napi::napi_status> {
        let f: Box<dyn FnOnce(TaskContext)> = Box::new(f);

        let status = unsafe {
            napi::napi_call_threadsafe_function(
                self.func,
                Box::into_raw(Box::new(f)) as *mut _,

                // TODO: Should this be user controllable?
                napi::napi_threadsafe_function_call_mode::napi_tsfn_blocking,
            )
        };

        match status {
            napi::napi_status::napi_ok => Ok(()),

            // TODO: Proper error enum
            _ => Err(status),
        }
    }

    fn unref<'a, C: Context<'a>>(&self, cx: &mut C) -> NeonResult<()> {
        let status = unsafe {
            napi::napi_unref_threadsafe_function(
                cx.env().to_raw(),
                self.func,
            )
        };

        if status != napi::napi_status::napi_ok {
            return cx.throw_error(
                "FIXME: Failed to unref ",
            );
        }

        Ok(())
    }
}

impl std::ops::Drop for EventQueueInternal {
    fn drop(&mut self) {
        let status = unsafe {
            napi::napi_release_threadsafe_function(
                self.func,
                napi::napi_threadsafe_function_release_mode::napi_tsfn_release,
            )
        };

        // TODO: Should we panic or ignore?
        assert_eq!(status, napi::napi_status::napi_ok);
    }
}

#[derive(Clone, Debug)]
pub struct EventQueue {
    internal: Arc<EventQueueInternal>,
}

impl EventQueue {
    pub fn new<'a, C: Context<'a>>(cx: &mut C) -> NeonResult<Self> {
        let internal = EventQueueInternal::new(cx)?;

        Ok(EventQueue {
            internal: Arc::new(internal),
        })
    }

    // TODO: Should we be conservative and take self mutably?
    pub fn send(&mut self, f: impl FnOnce(TaskContext)) -> Result<(), napi::napi_status> {
        self.internal.send(f)
    }

    pub(crate) fn unref<'a, C: Context<'a>>(self, cx: &mut C) -> NeonResult<Self> {
        self.internal.unref(cx)?;
        Ok(self)
    }
}

#[derive(Debug)]
struct PersistentInternal<T> {
    value: napi::napi_ref,
    queue: EventQueue,
    _marker: PhantomData<T>,
}

unsafe impl<T> Send for PersistentInternal<T> {}
unsafe impl<T> Sync for PersistentInternal<T> {}

impl<T> std::ops::Drop for PersistentInternal<T> {
    fn drop(&mut self) {
        let value = self.value.clone();

        self.queue.send(move |cx| {
            let mut result = MaybeUninit::uninit();
            let status = unsafe {
                napi::napi_reference_unref(
                    cx.env().to_raw(),
                    value as *mut _,
                    result.as_mut_ptr(),
                )
            };
        
            assert_eq!(status, napi::napi_status::napi_ok);            
        });
    }
}

// FIXME: This only appears to work on `Object` and `Function`
impl<T: Value> PersistentInternal<T> {
    pub fn new<'a, C: Context<'a>>(
        cx: &mut C,
        queue: EventQueue,
        v: Handle<T>,
    ) -> NeonResult<Self> {
        let mut value = MaybeUninit::uninit();

        let status = unsafe {
            napi::napi_create_reference(
                cx.env().to_raw(),
                v.to_raw(),
                1,
                value.as_mut_ptr(),
            )
        };

        if status != napi::napi_status::napi_ok {
            return cx.throw_error(
                "FIXME: Something bad happened creating reference",
            );
        }

        Ok(Self {
            value: unsafe { value.assume_init() },
            queue,
            _marker: PhantomData,
        })
    }

    pub fn deref<'a, C: Context<'a>>(&self, cx: &mut C) -> JsResult<'a, T> {
        let mut result = MaybeUninit::uninit();
        let status = unsafe {
            napi::napi_get_reference_value(
                cx.env().to_raw(),
                self.value,
                result.as_mut_ptr(),
            )
        };

        if status != napi::napi_status::napi_ok {
            return cx.throw_error(
                "FIXME: Something bad happened dereferencing reference",
            );
        }

        Ok(Handle::new_internal(T::from_raw(unsafe {
            result.assume_init()
        })))
    }
}

#[derive(Debug, Clone)]
pub struct Persistent<T>{
    internal: Arc<PersistentInternal<T>>,
}

impl<T: Value> Persistent<T> {
    pub fn new<'a, C: Context<'a>>(
        cx: &mut C,
        v: Handle<T>,
    ) -> NeonResult<Self> {
        let queue = cx.global_queue();

        PersistentInternal::new(cx, queue, v)
            .map(|v| Self {
                internal: Arc::new(v),
            })
    }

    // TODO: This consumes `self` for future optimizations
    // While we still have access to the main thread, we can drop the reference
    // and mark the structure as already dropped.
    pub fn deref<'a, C: Context<'a>>(self, cx: &mut C) -> JsResult<'a, T> {
        self.internal.deref(cx)
    }
}
