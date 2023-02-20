use pavex_builder::{f, router::GET, Blueprint};

pub fn handler() -> pavex_runtime::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET, "/home", f!(handler));
    bp
}
