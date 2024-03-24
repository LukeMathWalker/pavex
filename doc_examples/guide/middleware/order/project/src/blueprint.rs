use pavex::blueprint::Blueprint;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.nest(crate::core::blueprint());
    bp.nest_at("/pre_only", crate::pre_only::blueprint());
    bp.nest_at("/post_only", crate::post_only::blueprint());
    bp.nest_at("/wrap_only", crate::wrap_only::blueprint());
    bp.nest_at("/pre_and_post", crate::pre_and_post::blueprint());
    bp.nest_at("/post_and_wrap", crate::post_and_wrap::blueprint());
    bp.nest_at("/pre_and_wrap", crate::pre_and_wrap::blueprint());
    bp.nest_at("/order1", crate::order1::blueprint());
    bp.nest_at("/order2", crate::order2::blueprint());
    bp
}
