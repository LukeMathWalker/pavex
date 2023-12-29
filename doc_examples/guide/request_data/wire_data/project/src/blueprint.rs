use pavex::blueprint::Blueprint;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.nest(crate::head::blueprint());
    bp.nest(crate::body::blueprint());
    bp
}
