use std::sync::{Arc, Mutex};

use anyhow::Result;
use hyperlight_wasm::{LoadedWasmSandbox, SandboxBuilder, Snapshot};

use crate::bindings;
use crate::bindings::demo::policy::types::{Decision, Request};
use crate::bindings::demo::policy::{Handler, PolicyExports, PolicyImports, Types};

// The world imports only the `types` interface, which has no host functions,
// so the host state is empty.
struct State;

impl PolicyImports for State {
    type Types = State;
    fn types(&mut self) -> impl std::borrow::BorrowMut<Self::Types> {
        self
    }
}

impl Types for State {}

/// A loaded customer policy backed by a Hyperlight sandbox. One per customer.
/// Calls are serialized so the guest call and its restore stay atomic.
pub struct Policy {
    inner: Mutex<Inner>,
}

struct Inner {
    wrapped: bindings::PolicySandbox<State, LoadedWasmSandbox>,
    baseline: Arc<Snapshot>,
}

impl Policy {
    /// Load a policy component from raw wasm bytes and snapshot its clean
    /// post-load state.
    pub fn load(wasm: &[u8]) -> Result<Policy> {
        let mut proto = SandboxBuilder::new()
            .with_guest_input_buffer_size(16 * 1024 * 1024)
            .with_guest_heap_size(64 * 1024 * 1024)
            .with_guest_scratch_size(32 * 1024 * 1024)
            .build()?;
        let rt = bindings::register_host_functions(&mut proto, State);
        let runtime = proto.load_runtime()?;
        let mut loaded = runtime.load_module_from_buffer(wasm)?;
        let baseline = loaded.snapshot()?;
        let wrapped = bindings::PolicySandbox { sb: loaded, rt };
        Ok(Policy {
            inner: Mutex::new(Inner { wrapped, baseline }),
        })
    }

    /// Run the policy for one request, then restore the sandbox to its
    /// baseline so the next call starts clean.
    pub fn on_request(&self, req: Request) -> Result<Decision> {
        let mut inner = self.inner.lock().unwrap();
        let decision = PolicyExports::handler(&mut inner.wrapped).on_request(req);
        let baseline = inner.baseline.clone();
        inner.wrapped.sb.restore(baseline)?;
        Ok(decision)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bindings::demo::policy::types::Header;

    fn load(path: &str) -> Policy {
        Policy::load(&std::fs::read(path).unwrap()).unwrap()
    }

    fn req(path: &str, headers: Vec<Header>) -> Request {
        Request {
            method: "GET".into(),
            path: path.into(),
            headers,
        }
    }

    #[test]
    fn auth_check() {
        let p = load("../guests/auth_check/auth_check.aot");

        let no_auth = p.on_request(req("/orders/42", vec![])).unwrap();
        assert!(matches!(no_auth, Decision::Reject(r) if r.status == 401));

        let with_auth = p
            .on_request(req(
                "/orders/42",
                vec![Header {
                    name: "authorization".into(),
                    value: "Bearer x".into(),
                }],
            ))
            .unwrap();
        assert!(matches!(with_auth, Decision::Allow));
    }

    #[test]
    fn path_block() {
        let p = load("../guests/path_block/path_block.aot");

        assert!(matches!(
            p.on_request(req("/products", vec![])).unwrap(),
            Decision::Allow
        ));
        assert!(matches!(
            p.on_request(req("/admin/users", vec![])).unwrap(),
            Decision::Reject(r) if r.status == 403
        ));
    }
}
