use pavex::blueprint::constructor::Lifecycle;
use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::user::User::extract), Lifecycle::RequestScoped);
    bp.wrap(f!(crate::authentication::reject_anonymous));
    bp.route(GET, "/greet", f!(crate::routes::greet));
    bp
}
