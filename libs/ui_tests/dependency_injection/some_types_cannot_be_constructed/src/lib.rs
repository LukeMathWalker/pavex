use pavex::blueprint::Blueprint;
use pavex::connection::ConnectionInfo;
use pavex::f;
use pavex::request::{
    body::RawIncomingBody,
    path::{MatchedPathPattern, RawPathParams},
    RequestHead,
};
use pavex::router::AllowedMethods;

pub fn error() -> pavex::Error {
    todo!()
}

pub fn error_ref(_v: &RequestHead) -> &pavex::Error {
    todo!()
}

pub fn response() -> pavex::response::Response {
    todo!()
}

pub fn response_ref(_v: &RequestHead) -> &pavex::response::Response {
    todo!()
}

pub fn request_head() -> RequestHead {
    todo!()
}

pub fn request_head_ref(_v: &AllowedMethods) -> &RequestHead {
    todo!()
}

pub fn raw_incoming_body() -> RawIncomingBody {
    todo!()
}

pub fn raw_incoming_body_ref(_v: &AllowedMethods) -> &RawIncomingBody {
    todo!()
}

pub fn allowed_methods() -> AllowedMethods {
    todo!()
}

pub fn allowed_methods_ref(_v: &RequestHead) -> &AllowedMethods {
    todo!()
}

pub fn matched_path_pattern() -> MatchedPathPattern {
    todo!()
}

pub fn matched_path_pattern_ref(_v: &RequestHead) -> &MatchedPathPattern {
    todo!()
}

pub fn raw_path_params(_v: &RequestHead) -> RawPathParams {
    todo!()
}

pub fn raw_path_params_ref(_v: &RequestHead) -> &RawPathParams {
    todo!()
}

pub fn connection_info() -> ConnectionInfo {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.nest({
        let mut bp = Blueprint::new();
        bp.request_scoped(f!(crate::error));
        bp.request_scoped(f!(crate::response));
        bp.request_scoped(f!(crate::request_head));
        bp.request_scoped(f!(crate::allowed_methods));
        bp.request_scoped(f!(crate::raw_incoming_body));
        bp.request_scoped(f!(crate::matched_path_pattern));
        bp.request_scoped(f!(crate::raw_path_params));
        bp.request_scoped(f!(crate::connection_info));
        bp
    });
    bp.nest({
        let mut bp = Blueprint::new();
        bp.request_scoped(f!(crate::error_ref));
        bp.request_scoped(f!(crate::response_ref));
        bp.request_scoped(f!(crate::request_head_ref));
        bp.request_scoped(f!(crate::allowed_methods_ref));
        bp.request_scoped(f!(crate::raw_incoming_body_ref));
        bp.request_scoped(f!(crate::matched_path_pattern_ref));
        bp.request_scoped(f!(crate::raw_path_params_ref));
        bp
    });
    bp
}
