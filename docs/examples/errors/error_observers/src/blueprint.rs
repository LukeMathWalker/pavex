//! px:registration
use crate::logger::EMIT_ERROR_LOG;
use pavex::Blueprint;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.error_observer(EMIT_ERROR_LOG); // px::ann:1
                                       // px::skip:start
    bp.error_observer(crate::injection::ENRICH_ROOT_SPAN);
    bp.import(pavex::blueprint::from![crate]);
    bp.routes(pavex::blueprint::from![crate]);
    bp
    // px::skip:end
}
