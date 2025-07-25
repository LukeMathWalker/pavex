use dep::Logger;
use pavex::{blueprint::from, Blueprint};

#[pavex::get(path = "/home")]
pub fn handler(_logger: Logger) -> pavex::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    dep::add_logger(&mut bp);
    bp.routes(from![crate]);
    bp
}
