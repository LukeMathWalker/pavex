use pavex::blueprint::router::{POST, PUT};
use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;

pub(crate) fn users_bp() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(POST, "/users", f!(crate::routes::users::signup));
    bp.route(POST, "/users/login", f!(crate::routes::users::login));
    bp.route(GET, "/user", f!(crate::routes::users::get_user));
    bp.route(PUT, "/user", f!(crate::routes::users::update_user));
    bp
}

mod get_user;
mod login;
mod signup;
mod update_user;

pub use get_user::*;
pub use login::*;
pub use signup::*;
pub use update_user::*;
