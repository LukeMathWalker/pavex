use dep::Logger;
use pavex::blueprint::{constructor::Lifecycle, from, Blueprint};
use pavex::f;

#[pavex::get(path = "/home")]
pub fn handler(_logger: Logger) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(new_logger), Lifecycle::Singleton);
    bp.routes(from![crate]);
    bp
}
