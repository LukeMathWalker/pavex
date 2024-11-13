use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;

pub fn handler() -> String {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    // Invalid domain!
    bp.domain("s{.com").nest({
        let mut bp = Blueprint::new();
        bp.route(GET, "/", f!(crate::handler));
        bp
    });
    bp
}
