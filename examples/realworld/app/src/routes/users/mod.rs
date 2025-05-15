use pavex::blueprint::router::{POST, PUT};
use pavex::blueprint::{Blueprint, router::GET};
use pavex::f;

pub(crate) fn users_bp() -> Blueprint {
    let mut bp = Blueprint::new();

    bp.route(POST, "/users", f!(self::signup))
        .error_handler(f!(self::SignupError::into_response));
    bp.route(POST, "/users/login", f!(self::login))
        .error_handler(f!(self::LoginError::into_response));
    bp.route(GET, "/user", f!(self::get_user));
    bp.route(PUT, "/user", f!(self::update_user));
    bp
}

mod endpoints;
mod password;

pub use endpoints::*;
