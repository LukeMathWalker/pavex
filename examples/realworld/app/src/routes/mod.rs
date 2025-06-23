use pavex::blueprint::{Blueprint, from};
use status::PING;
use tags::LIST_TAGS;

pub mod articles;
pub mod profiles;
pub mod status;
pub mod tags;
pub mod users;

pub fn router(bp: &mut Blueprint) {
    bp.prefix("/articles").nest({
        let mut bp = Blueprint::new();
        bp.routes(from![crate::routes::articles]);
        bp
    });
    bp.prefix("/profiles").nest({
        let mut bp = Blueprint::new();
        bp.routes(from![crate::routes::profiles]);
        bp
    });
    bp.nest({
        let mut bp = Blueprint::new();
        bp.routes(from![crate::routes::users]);
        bp
    });
    bp.route(PING);
    bp.route(LIST_TAGS);
}
