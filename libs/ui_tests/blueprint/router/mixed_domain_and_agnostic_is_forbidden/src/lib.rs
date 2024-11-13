use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;

pub fn handler() -> String {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    // Domain-specific
    bp.domain("company.com").nest({
        let mut bp = Blueprint::new();
        bp.route(GET, "/", f!(crate::handler));
        bp
    });
    // Domain-agnostic
    bp.route(GET, "/login", f!(crate::handler));
    bp
}
