use pavex_builder::{f, router::GET, Blueprint};

pub struct Streamer;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET, "/home", f!(crate::Streamer));
    bp
}
