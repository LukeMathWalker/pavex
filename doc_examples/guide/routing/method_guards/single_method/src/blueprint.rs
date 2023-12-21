use pavex::blueprint::{router::GET /* (1)! */, Blueprint};
use pavex::f;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET, "/greet", f!(crate::routes::greet));
    bp
}
