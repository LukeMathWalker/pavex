use dep::{new_logger, Logger};
use pavex_builder::{f, Blueprint, Lifecycle};

pub fn handler(logger: Logger) -> pavex_runtime::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(new_logger), Lifecycle::Singleton);
    bp.route(f!(crate::handler), "/home");
    bp
}
