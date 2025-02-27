use pavex::blueprint::Blueprint;
use pavex::cookie::CookieKit;
use pavex_session_memory_store::InMemorySessionKit;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();

    InMemorySessionKit::new()
        .with_default_config()
        .register(&mut bp);
    // Sessions are built on top of cookies,
    // so you need to set those up too.
    // Order is important here!
    CookieKit::new().register(&mut bp);
    bp
}
