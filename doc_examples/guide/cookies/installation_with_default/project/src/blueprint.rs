use pavex::blueprint::Blueprint;
use pavex::cookie::CookieKit;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    CookieKit::new()
        .with_default_processor_config()
        .register(&mut bp);
    bp
}
