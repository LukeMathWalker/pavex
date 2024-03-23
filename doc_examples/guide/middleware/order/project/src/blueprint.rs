use pavex::blueprint::Blueprint;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.nest(crate::core::blueprint());
    bp.nest_at("/order1", crate::order1::blueprint());
    bp.nest_at("/order2", crate::order2::blueprint());
    bp
}
