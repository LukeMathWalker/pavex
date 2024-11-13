use pavex::blueprint::router::GET;
use pavex::blueprint::Blueprint;
use pavex::f;

pub fn bp() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.prefix("/homes").nest({
        /* (1)! */
        let mut bp = Blueprint::new();
        bp.route(GET, "/", f!(super::list_homes));
        bp.route(GET, "/{id}", f!(super::get_home));
        bp
    });
    bp.route(GET, "/", f!(super::index));
    bp
}
