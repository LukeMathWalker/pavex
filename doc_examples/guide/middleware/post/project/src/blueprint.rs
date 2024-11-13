use pavex::blueprint::Blueprint;
use pavex::f;

pub fn root_span() -> pavex_tracing::RootSpan {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.request_scoped(f!(self::root_span));
    bp.nest(crate::core::blueprint());
    bp.prefix("/fallible").nest(crate::fallible::blueprint());
    bp
}
