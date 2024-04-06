use pavex::blueprint::Blueprint;
use pavex::cookie::CookieKit;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    CookieKit::new().register(&mut bp);
    bp
}
