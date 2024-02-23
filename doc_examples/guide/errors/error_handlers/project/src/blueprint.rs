use pavex::blueprint::Blueprint;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.nest_at("/core", crate::core::blueprint());
    bp.nest_at("/universal", crate::universal::blueprint());
    bp
}
