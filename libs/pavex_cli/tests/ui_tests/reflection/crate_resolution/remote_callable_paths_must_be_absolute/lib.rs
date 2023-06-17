use dep::{new_logger, Logger};
use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;

pub fn handler(logger: Logger) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(new_logger), Lifecycle::Singleton);
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
