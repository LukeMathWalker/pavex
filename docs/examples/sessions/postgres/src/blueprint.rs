//! px:postgres
use pavex::{Blueprint, blueprint::from, cookie::INJECT_RESPONSE_COOKIES};
use pavex_session::FINALIZE_SESSION;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();

    bp.import(from![
        // Other imports [..]
        pavex,                        // px::ann:1
        pavex_session,                // px::ann:2
        pavex_session_sqlx::postgres  // px::ann:3
    ]);
    bp.post_process(FINALIZE_SESSION); // px::ann:4
    bp.post_process(INJECT_RESPONSE_COOKIES); // px::ann:5
    // px::skip:start
    bp.prefix("/ops").routes(from![crate::ops]);
    bp.import(from![crate]);
    bp
    // px::skip:end
}
