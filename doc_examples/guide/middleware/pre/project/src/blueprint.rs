use pavex::blueprint::Blueprint;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.nest(crate::core::blueprint());
    bp.prefix("/fallible").nest(crate::fallible::blueprint());
    bp
}
