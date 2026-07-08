use std::cell::RefCell;
use std::ffi::{c_char, CStr, CString};
use std::ptr;

use crate::bindings::demo::policy::types::{Decision, Header, Request};
use crate::engine::Policy;

/// One HTTP header, both fields null-terminated UTF-8.
#[repr(C)]
pub struct FfiHeader {
    pub name: *const c_char,
    pub value: *const c_char,
}

/// The request the gateway passes in. Strings are null-terminated UTF-8.
#[repr(C)]
pub struct FfiRequest {
    pub method: *const c_char,
    pub path: *const c_char,
    pub headers: *const FfiHeader,
    pub headers_len: usize,
}

/// The policy decision written back to the caller. `kind` is 0 for allow,
/// 1 for reject. `status` and `message` are meaningful only for reject.
/// `message` is owned by the callee, free it with `policy_decision_free`.
#[repr(C)]
pub struct FfiDecision {
    pub kind: i32,
    pub status: u16,
    pub message: *mut c_char,
}

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

fn set_last_error(msg: String) {
    LAST_ERROR.with(|e| *e.borrow_mut() = CString::new(msg).ok());
}

/// Return the last error message for this thread, or null if none.
#[no_mangle]
pub extern "C" fn policy_last_error() -> *const c_char {
    LAST_ERROR.with(|e| match &*e.borrow() {
        Some(s) => s.as_ptr(),
        None => ptr::null(),
    })
}

/// Load a policy from wasm bytes. Returns an opaque handle, or null on error
/// (see `policy_last_error`).
///
/// # Safety
/// `wasm` must point to `len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn policy_engine_load(wasm: *const u8, len: usize) -> *mut Policy {
    if wasm.is_null() {
        set_last_error("null wasm pointer".into());
        return ptr::null_mut();
    }
    let bytes = std::slice::from_raw_parts(wasm, len);
    match Policy::load(bytes) {
        Ok(policy) => Box::into_raw(Box::new(policy)),
        Err(e) => {
            set_last_error(format!("load failed: {e}"));
            ptr::null_mut()
        }
    }
}

/// Run the policy for one request, writing the decision into `out`.
/// Returns 0 on success, nonzero on error (see `policy_last_error`).
///
/// # Safety
/// `handle` must come from `policy_engine_load`. `req` and `out` must be
/// valid pointers. Strings in `req` must be null-terminated UTF-8.
#[no_mangle]
pub unsafe extern "C" fn policy_on_request(
    handle: *mut Policy,
    req: *const FfiRequest,
    out: *mut FfiDecision,
) -> i32 {
    if handle.is_null() || req.is_null() || out.is_null() {
        set_last_error("null argument".into());
        return -1;
    }
    let policy = &*handle;
    let request = match build_request(&*req) {
        Ok(r) => r,
        Err(e) => {
            set_last_error(e);
            return -1;
        }
    };
    match policy.on_request(request) {
        Ok(decision) => {
            *out = to_ffi_decision(decision);
            0
        }
        Err(e) => {
            set_last_error(format!("on_request failed: {e}"));
            -1
        }
    }
}

/// Free the `message` owned by a decision written by `policy_on_request`.
///
/// # Safety
/// `out` must point to a decision written by `policy_on_request`.
#[no_mangle]
pub unsafe extern "C" fn policy_decision_free(out: *mut FfiDecision) {
    if out.is_null() {
        return;
    }
    let decision = &mut *out;
    if !decision.message.is_null() {
        drop(CString::from_raw(decision.message));
        decision.message = ptr::null_mut();
    }
}

/// Free a policy handle.
///
/// # Safety
/// `handle` must come from `policy_engine_load` and not be used afterwards.
#[no_mangle]
pub unsafe extern "C" fn policy_engine_free(handle: *mut Policy) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

unsafe fn build_request(req: &FfiRequest) -> Result<Request, String> {
    let method = cstr(req.method)?;
    let path = cstr(req.path)?;
    let mut headers = Vec::new();
    if !req.headers.is_null() {
        for h in std::slice::from_raw_parts(req.headers, req.headers_len) {
            headers.push(Header {
                name: cstr(h.name)?,
                value: cstr(h.value)?,
            });
        }
    }
    Ok(Request {
        method,
        path,
        headers,
    })
}

unsafe fn cstr(p: *const c_char) -> Result<String, String> {
    if p.is_null() {
        return Err("null string".into());
    }
    CStr::from_ptr(p)
        .to_str()
        .map(|s| s.to_string())
        .map_err(|e| e.to_string())
}

fn to_ffi_decision(decision: Decision) -> FfiDecision {
    match decision {
        Decision::Allow => FfiDecision {
            kind: 0,
            status: 0,
            message: ptr::null_mut(),
        },
        Decision::Reject(rejection) => FfiDecision {
            kind: 1,
            status: rejection.status,
            message: CString::new(rejection.message)
                .unwrap_or_default()
                .into_raw(),
        },
    }
}
