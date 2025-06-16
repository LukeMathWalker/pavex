use pavex::blueprint::{from, Blueprint};
use pavex::http::StatusCode;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(FIRST);
    bp.prefix("/first").nest(first_bp());
    bp
}

fn first_bp() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(SECOND);
    bp.prefix("/second").nest(second_bp());
    bp
}

fn second_bp() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(THIRD);
    bp.prefix("/third").nest(third_bp());
    bp
}

fn third_bp() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.routes(from![crate]);
    bp
}

#[pavex::request_scoped]
pub fn first() -> u16 {
    todo!()
}

#[pavex::request_scoped]
pub fn second(_x: u16) -> u32 {
    todo!()
}

#[pavex::request_scoped]
pub fn third(_x: u32) -> String {
    todo!()
}

#[pavex::get(path = "/home")]
pub fn handler(_x: String) -> StatusCode {
    todo!()
}
