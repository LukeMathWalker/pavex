use pavex::blueprint::{from, router::GET, Blueprint};
use pavex::f;

#[pavex::get(path = "/")]
pub fn non_domain() -> String {
    todo!()
}

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
    bp.routes(from![crate]);
    bp
}
