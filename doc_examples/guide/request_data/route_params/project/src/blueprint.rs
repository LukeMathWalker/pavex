use pavex::blueprint::Blueprint;
use pavex::request::route::RouteParams;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    RouteParams::register(&mut bp);
    bp.nest(crate::route_params::blueprint());
    bp
}
