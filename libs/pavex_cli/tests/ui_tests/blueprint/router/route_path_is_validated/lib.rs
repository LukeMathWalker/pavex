use pavex_builder::{f, router::GET, Blueprint};

pub fn handler() -> String {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    // Empty path
    bp.route(GET, "", f!(crate::handler));
    // Path does not start with a `/`
    bp.route(GET, "api", f!(crate::handler));
    bp
}
