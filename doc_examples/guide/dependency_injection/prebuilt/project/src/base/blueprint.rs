use pavex::blueprint::router::GET;
use pavex::blueprint::Blueprint;
use pavex::{f, t};

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.prebuilt(t!(super::A));
    bp.route(GET, "/", f!(super::handler));
    bp
}
