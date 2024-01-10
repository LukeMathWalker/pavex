use pavex::blueprint::Blueprint;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.nest(crate::core::blueprint());
    bp.nest(crate::logging::blueprint());
    bp
}
