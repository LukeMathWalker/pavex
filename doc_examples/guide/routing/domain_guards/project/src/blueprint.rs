use pavex::blueprint::Blueprint;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.nest(crate::intro::bp());
    bp.prefix("/static").nest(crate::static_::bp());
    bp.prefix("/dynamic").nest(crate::dynamic::bp());
    bp.prefix("/multi").nest(crate::multi::bp());
    bp.prefix("/catch_all").nest(crate::catch_all::bp());
    bp.prefix("/fallback").nest(crate::fallback::bp());
    bp
}
