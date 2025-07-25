use pavex::connection::ConnectionInfo;
use pavex::request::{
    body::RawIncomingBody,
    path::{MatchedPathPattern, RawPathParams},
    RequestHead,
};
use pavex::router::AllowedMethods;
use pavex::Blueprint;

#[pavex::fallback]
pub fn handler(
    _info: &ConnectionInfo,
    _head: &RequestHead,
    _body: &RawIncomingBody,
    _methods: &AllowedMethods,
    _pattern: &MatchedPathPattern,
    _params: &RawPathParams,
) -> pavex::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.prefix("/nested").nest({
        let mut bp = Blueprint::new();
        bp.fallback(HANDLER);
        bp
    });
    bp.fallback(HANDLER);
    bp
}
