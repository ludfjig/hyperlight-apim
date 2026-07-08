#[allow(warnings)]
mod bindings;

use bindings::exports::demo::policy::handler::Guest;
use bindings::demo::policy::types::{Decision, Request};

struct Component;

impl Guest for Component {
    fn on_request(_req: Request) -> Decision {
        panic!("crasher policy always panics");
    }
}

bindings::export!(Component with_types_in bindings);
