use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;

pub struct Streamer;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET, "/home", f!(crate::Streamer));
    bp
}
