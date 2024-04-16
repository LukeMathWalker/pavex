use pavex::blueprint::Blueprint;
use pavex::request::body::UrlEncodedBody;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    UrlEncodedBody::register(&mut bp); // (1)!
    pavex::request::body::BufferedBody::register(&mut bp);
    pavex::request::body::BodySizeLimit::register(&mut bp);

    bp.nest(crate::urlencoded::blueprint());
    bp
}
