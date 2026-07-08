#[allow(warnings)]
mod bindings;

use bindings::exports::demo::policy::handler::Guest;
use bindings::demo::policy::types::{Decision, Rejection, Request};

struct Component;

impl Guest for Component {
    fn on_request(req: Request) -> Decision {
        let has_auth = req
            .headers
            .iter()
            .any(|h| h.name.eq_ignore_ascii_case("authorization"));

        if has_auth {
            Decision::Allow
        } else {
            Decision::Reject(Rejection {
                status: 401,
                message: "missing authorization header".to_string(),
            })
        }
    }
}

bindings::export!(Component with_types_in bindings);
