use pavex::blueprint::{Blueprint, router::GET};
use pavex::f;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();

    bp.post_process(f!(crate::post1));
    bp.post_process(f!(crate::post2));
    bp.route(GET, "/", f!(super::handler));

    bp
}
