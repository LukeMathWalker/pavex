use pavex::blueprint::Blueprint;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.nest(crate::base::blueprint());
    bp.nest(crate::skip::blueprint());
    bp.nest(crate::tweak::blueprint());
    bp.nest(crate::replace::blueprint());
    bp
}
