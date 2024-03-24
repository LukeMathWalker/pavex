use pavex::blueprint::{Blueprint, router::GET};
use pavex::f;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();

    bp.pre_process(f!(crate::pre1));
    bp.wrap(f!(crate::wrap1));
    bp.pre_process(f!(crate::pre2));
    bp.wrap(f!(crate::wrap2));
    bp.pre_process(f!(crate::pre3));
    bp.route(GET, "/", f!(super::handler));

    bp
}
