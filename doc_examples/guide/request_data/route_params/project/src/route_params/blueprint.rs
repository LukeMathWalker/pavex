use pavex::blueprint::router::GET;
use pavex::blueprint::Blueprint;
use pavex::f;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(
        GET,
        "/users/:id", /* (1)! */
        f!(crate::route_params::handler),
    );
    bp
}
