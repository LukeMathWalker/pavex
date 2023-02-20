use pavex_builder::{f, Blueprint};

pub fn handler() -> pavex_runtime::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(f!(handler), "/home");
    bp
}
