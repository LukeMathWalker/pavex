use pavex::blueprint::Blueprint;
use pavex::f;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.request_scoped(f!(super::parse));
    {
        use pavex::request::body::{BodySizeLimit, BufferedBody};
        BufferedBody::register(&mut bp);
        BodySizeLimit::register(&mut bp);
    }
    bp
}
