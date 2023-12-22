use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.nest(crate::functions::blueprint());
    bp.nest(crate::static_methods::blueprint());
    bp.nest(crate::non_static_methods::blueprint());
    bp.nest(crate::trait_methods::blueprint());
    bp.nest(crate::output::blueprint());
    bp.nest(crate::input::blueprint());
    bp.route(GET, "/greet", f!(crate::routes::greet));
    bp
}
