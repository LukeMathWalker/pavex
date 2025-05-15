use pavex::blueprint::from;
use pavex::blueprint::Blueprint;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![pavex]);
    bp.nest(crate::json::blueprint());
    bp
}
