use pavex::{blueprint::Blueprint, cookie::CookieKit};
use pavex_session_sqlx::SqliteSessionKit;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    SqliteSessionKit::new().register(&mut bp);
    // Sessions are built on top of cookies,
    // so you need to set those up too.
    // Order is important here!
    CookieKit::new().register(&mut bp);

    bp.prebuilt(pavex::t!(sqlx::pool::Pool<sqlx::Sqlite>));
    bp.prefix("/ops").nest(crate::ops::blueprint());
    bp
}
