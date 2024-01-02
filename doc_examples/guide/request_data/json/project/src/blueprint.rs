use pavex::blueprint::Blueprint;
use pavex::request::body::{BodySizeLimit, BufferedBody, JsonBody};

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();

    JsonBody::register(&mut bp);
    BufferedBody::register(&mut bp);
    BodySizeLimit::register(&mut bp);

    bp.nest(crate::json::blueprint());
    bp
}
