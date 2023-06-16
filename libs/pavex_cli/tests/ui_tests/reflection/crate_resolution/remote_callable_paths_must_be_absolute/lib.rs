use dep::{new_logger, Logger};
use pavex_builder::{constructor::Lifecycle, f, router::GET, Blueprint};

pub fn handler(logger: Logger) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(new_logger), Lifecycle::Singleton);
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
