use pavex::blueprint::router::GET;
use pavex::blueprint::Blueprint;
use pavex::f;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();

    bp.pre_process(f!(crate::pre1));
    bp.post_process(f!(crate::post1));
    bp.wrap(f!(crate::wrap1));
    bp.pre_process(f!(crate::pre2));
    bp.post_process(f!(crate::post2));
    bp.route(GET, "/", f!(super::handler));

    bp
}
