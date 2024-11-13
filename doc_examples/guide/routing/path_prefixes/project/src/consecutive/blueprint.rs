use pavex::blueprint::router::GET;
use pavex::blueprint::Blueprint;
use pavex::f;

pub fn bp() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.prefix("/prefix").nest({
        let mut bp = Blueprint::new();
        bp.route(GET, "//path", f!(super::handler));
        bp
    });
    bp
}
