use pavex::blueprint::from;
use pavex::blueprint::Blueprint;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![pavex]);
    bp.nest(crate::buffered_body::blueprint());
    bp.nest(crate::custom_limit::blueprint());
    bp.nest(crate::no_limit::blueprint());
    bp
}
