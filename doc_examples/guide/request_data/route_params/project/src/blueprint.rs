use pavex::blueprint::Blueprint;
use pavex::request::path::PathParams;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    PathParams::register(&mut bp);
    bp.nest(crate::route_params::blueprint());
    bp
}
