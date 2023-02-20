use pavex_builder::{f, Blueprint, Lifecycle};

pub fn static_str() -> &'static str {
    todo!()
}

pub fn handler(_x: &'static str) -> pavex_runtime::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::static_str), Lifecycle::Singleton);
    bp.route(f!(crate::handler), "/handler");
    bp
}
