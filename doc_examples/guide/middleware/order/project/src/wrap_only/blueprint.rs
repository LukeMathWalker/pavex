use pavex::blueprint::{Blueprint, router::GET};
use pavex::f;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();

    bp.wrap(f!(crate::wrap1));
    bp.wrap(f!(crate::wrap2));
    bp.route(GET, "/", f!(super::handler));

    bp
}
