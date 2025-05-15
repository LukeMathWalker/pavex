use pavex::blueprint::{router::GET, Blueprint};
use pavex::response::Response;
use pavex::{f, wrap};

pub fn mw() -> Response {
    todo!()
}

#[wrap]
pub fn mw_1() -> Response {
    todo!()
}

pub fn handler() -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.wrap(f!(crate::mw));
    bp.wrap(MW_1_ID);
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
