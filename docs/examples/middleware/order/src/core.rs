//! px:registration
use pavex::Blueprint;
// px::skip:start
use super::GET_INDEX;
use crate::{POST_1, POST_2, PRE_1, PRE_2, WRAP_1};
// px::skip:end

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();

    bp.pre_process(PRE_1);
    bp.post_process(POST_1);
    bp.wrap(WRAP_1);
    bp.pre_process(PRE_2);
    bp.post_process(POST_2);
    bp.route(GET_INDEX);
    bp // px::skip
}
