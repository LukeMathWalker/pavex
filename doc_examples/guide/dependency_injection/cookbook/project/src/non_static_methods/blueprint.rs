use pavex::blueprint::constructor::Lifecycle;
use pavex::blueprint::Blueprint;
use pavex::f;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(super::UserStore::retrieve), Lifecycle::RequestScoped);
    bp.constructor(f!(super::UserStore::new), Lifecycle::Singleton);
    bp
}
