use pavex_builder::{f, Blueprint};

pub struct Streamer;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(f!(crate::Streamer), "/home");
    bp
}
