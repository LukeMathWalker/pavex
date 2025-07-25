//! px:installation
use pavex::{Blueprint, blueprint::from, cookie::INJECT_RESPONSE_COOKIES};

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![
        // Other imports [...]
        pavex // px::hl px::ann:1
    ]);
    bp.post_process(INJECT_RESPONSE_COOKIES); // px::hl px::ann:2
    // px:installation:skip:start
    bp.prefix("/get_one").routes(from![crate::get_one]);
    bp.prefix("/get_all").routes(from![crate::get_all]);
    bp.prefix("/insert").routes(from![crate::insert]);
    bp.prefix("/delete").routes(from![crate::delete]);
    bp
    // px:installation:skip:end
}
