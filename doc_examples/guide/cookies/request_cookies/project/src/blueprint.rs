use pavex::blueprint::Blueprint;
use pavex::cookie::CookieKit;
use pavex::f;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    CookieKit::new().register(&mut bp);
    bp.singleton(f!(
        <pavex::cookie::ProcessorConfig as std::default::Default>::default
    ));
    bp.prefix("/core").nest(crate::core::blueprint());
    bp.prefix("/multiple").nest(crate::multiple::blueprint());
    bp
}
