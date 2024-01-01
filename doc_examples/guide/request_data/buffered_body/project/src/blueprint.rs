use pavex::blueprint::Blueprint;
use pavex::request::body::{BodySizeLimit, BufferedBody};

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();

    BufferedBody::register(&mut bp);
    BodySizeLimit::register(&mut bp);

    bp.nest(crate::buffered_body::blueprint());
    bp
}
