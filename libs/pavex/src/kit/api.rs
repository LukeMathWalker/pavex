use crate::blueprint::Blueprint;
use crate::request::body::{BodySizeLimit, BufferedBody, JsonBody};
use crate::request::path::PathParams;
use crate::request::query::QueryParams;

#[derive(Clone)]
pub struct ApiKit {}

impl ApiKit {
    pub fn new() -> Self {
        Self {}
    }

    pub fn register(&self, bp: &mut Blueprint) {
        PathParams::register(bp);
        QueryParams::register(bp);
        JsonBody::register(bp);
        BufferedBody::register(bp);
        BodySizeLimit::register(bp);
    }
}
