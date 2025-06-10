use pavex::blueprint::{from, Blueprint};
use pavex::cookie::CookieKit;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![pavex]);
    CookieKit::new().register(&mut bp);
    bp.prefix("/core").nest(crate::core::blueprint());
    bp.prefix("/multiple").nest(crate::multiple::blueprint());
    bp
}
