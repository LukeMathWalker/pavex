use pavex::blueprint::Blueprint;
use pavex::request::query::QueryParams;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    QueryParams::register(&mut bp);
    bp.nest(crate::query_params::blueprint());
    bp
}
