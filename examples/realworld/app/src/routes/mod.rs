use pavex::{Blueprint, blueprint::from};
use status::PING;
use tags::LIST_TAGS;

pub mod articles;
pub mod profiles;
pub mod status;
pub mod tags;
pub mod users;

pub fn router(bp: &mut Blueprint) {
    bp.prefix("/articles")
        .routes(from![crate::routes::articles]);
    bp.prefix("/profiles")
        .routes(from![crate::routes::profiles]);
    bp.routes(from![crate::routes::users]);
    bp.route(PING);
    bp.route(LIST_TAGS);
}
