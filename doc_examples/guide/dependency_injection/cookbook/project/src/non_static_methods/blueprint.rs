use pavex::blueprint::Blueprint;
use pavex::f;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.request_scoped(f!(super::UserStore::retrieve));
    bp.request_scoped(f!(super::UserStore::new));
    bp
}
