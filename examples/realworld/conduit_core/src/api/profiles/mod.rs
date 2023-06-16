use pavex::blueprint::router::{DELETE, POST};
use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;

pub(crate) fn profiles_bp() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET, "/:username", f!(crate::api::profiles::get_user));
    bp.route(
        POST,
        "/:username/follow",
        f!(crate::api::profiles::follow_user),
    );
    bp.route(
        DELETE,
        "/:username/follow",
        f!(crate::api::profiles::unfollow_user),
    );
    bp
}

mod follow_user;
mod get_user;
mod unfollow_user;

pub use follow_user::*;
pub use get_user::*;
pub use unfollow_user::*;
