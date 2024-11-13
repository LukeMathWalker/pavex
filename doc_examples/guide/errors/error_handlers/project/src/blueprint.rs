use pavex::blueprint::Blueprint;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.prefix("/core").nest(crate::core::blueprint());
    bp.prefix("/universal").nest(crate::universal::blueprint());
    bp
}
