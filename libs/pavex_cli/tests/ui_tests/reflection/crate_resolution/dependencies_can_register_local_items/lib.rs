use dep::{new_logger, Logger};
use pavex_builder::{constructor::Lifecycle, f, router::GET, Blueprint};

pub fn handler(logger: Logger) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    dep::add_logger(&mut bp);
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
