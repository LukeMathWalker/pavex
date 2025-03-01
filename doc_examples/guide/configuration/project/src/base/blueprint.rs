use pavex::blueprint::router::GET;
use pavex::blueprint::Blueprint;
use pavex::{f, t};

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.config("database", t!(super::DatabaseConfig));
    bp.route(GET, "/", f!(super::handler));
    bp
}
