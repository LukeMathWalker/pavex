use pavex::blueprint::Blueprint;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.prefix("/in_memory").nest(crate::in_memory::blueprint());
    bp
}
