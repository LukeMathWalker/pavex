//! px:post_only
use crate::{GET_INDEX, POST_1, POST_2}; // px::skip
use pavex::Blueprint;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();

    bp.post_process(POST_1);
    bp.post_process(POST_2);
    bp.route(GET_INDEX);
    bp // px::skip
}
