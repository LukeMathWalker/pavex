use pavex::blueprint::Blueprint;
use pavex::cookie::CookieKit;
use pavex::f;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    CookieKit::new().register(&mut bp);
    bp.singleton(f!(<pavex::cookie::ProcessorConfig as std::default::Default>::default));
    bp.nest_at("/core", crate::core::blueprint());
    bp.nest_at("/multiple", crate::multiple::blueprint());
    bp
}
