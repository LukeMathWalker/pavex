use pavex::Blueprint;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.nest(crate::core::blueprint());
    bp.prefix("/pre_only").nest(crate::pre_only::blueprint());
    bp.prefix("/post_only").nest(crate::post_only::blueprint());
    bp.prefix("/wrap_only").nest(crate::wrap_only::blueprint());
    bp.prefix("/pre_and_post")
        .nest(crate::pre_and_post::blueprint());
    bp.prefix("/post_and_wrap")
        .nest(crate::post_and_wrap::blueprint());
    bp.prefix("/pre_and_wrap")
        .nest(crate::pre_and_wrap::blueprint());
    bp.prefix("/order1").nest(crate::order1::blueprint());
    bp.prefix("/order2").nest(crate::order2::blueprint());
    bp
}
