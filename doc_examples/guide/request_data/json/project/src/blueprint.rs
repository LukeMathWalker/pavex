use pavex::blueprint::Blueprint;
use pavex::request::body::JsonBody;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    JsonBody::register(&mut bp); // (1)!
    pavex::request::body::BufferedBody::register(&mut bp);
    pavex::request::body::BodySizeLimit::register(&mut bp);

    bp.nest(crate::json::blueprint());
    bp
}
