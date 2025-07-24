use pavex::{blueprint::from, Blueprint};

#[derive(Clone)]
pub struct Streamer {
    pub a: usize,
    pub b: isize,
}

#[pavex::singleton]
pub fn streamer() -> Streamer {
    todo!()
}

#[pavex::get(path = "/home")]
pub fn route_handler(Streamer { a: _a, b: _b }: &Streamer) -> pavex::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
