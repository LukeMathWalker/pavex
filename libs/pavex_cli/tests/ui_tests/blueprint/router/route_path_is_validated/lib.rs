use pavex_builder::{f, router::GET, Blueprint};

pub fn handler() -> String {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    // Empty path is accepted
    bp.route(GET, "", f!(crate::handler));
    // If the path is not empty, it *must* start with a `/`
    bp.route(GET, "api", f!(crate::handler));
    bp
}
