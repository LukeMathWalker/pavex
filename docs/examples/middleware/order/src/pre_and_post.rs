//! px:pre_and_post
use crate::{GET_INDEX, POST_1, POST_2, PRE_1, PRE_2};
use pavex::Blueprint;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();

    bp.pre_process(PRE_1);
    bp.post_process(POST_1);
    bp.post_process(POST_2);
    bp.pre_process(PRE_2);
    bp.route(GET_INDEX);
    bp // px::skip
}
