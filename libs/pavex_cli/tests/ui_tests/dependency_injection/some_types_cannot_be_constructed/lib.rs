use pavex::blueprint::constructor::Lifecycle;
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

pub fn error_ref(request_head: &RequestHead) -> &pavex::Error {
    todo!()
}

pub fn response() -> pavex::response::Response {
    todo!()
}

pub fn response_ref(request_head: &RequestHead) -> &pavex::response::Response {
    todo!()
}

pub fn request_head() -> RequestHead {
    todo!()
}

pub fn request_head_ref(request_head: &AllowedMethods) -> &RequestHead {
    todo!()
}

pub fn raw_incoming_body() -> RawIncomingBody {
    todo!()
}

pub fn raw_incoming_body_ref(request_head: &AllowedMethods) -> &RawIncomingBody {
    todo!()
}

pub fn allowed_methods() -> AllowedMethods {
    todo!()
}

pub fn allowed_methods_ref(request_head: &RequestHead) -> &AllowedMethods {
    todo!()
}

pub fn matched_path_pattern() -> MatchedPathPattern {
    todo!()
}

pub fn matched_path_pattern_ref(request_head: &RequestHead) -> &MatchedPathPattern {
    todo!()
}

pub fn raw_path_params(request_head: &RequestHead) -> RawPathParams {
    todo!()
}

pub fn raw_path_params_ref(request_head: &RequestHead) -> &RawPathParams {
    todo!()
}

pub fn connection_info() -> ConnectionInfo {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.nest({
        let mut bp = Blueprint::new();
        bp.constructor(f!(crate::error), Lifecycle::RequestScoped);
        bp.constructor(f!(crate::response), Lifecycle::RequestScoped);
        bp.constructor(f!(crate::request_head), Lifecycle::RequestScoped);
        bp.constructor(f!(crate::allowed_methods), Lifecycle::RequestScoped);
        bp.constructor(f!(crate::raw_incoming_body), Lifecycle::RequestScoped);
        bp.constructor(f!(crate::matched_path_pattern), Lifecycle::RequestScoped);
        bp.constructor(f!(crate::raw_path_params), Lifecycle::RequestScoped);
        bp.constructor(f!(crate::connection_info), Lifecycle::RequestScoped);
        bp
    });
    bp.nest({
        let mut bp = Blueprint::new();
        bp.constructor(f!(crate::error_ref), Lifecycle::RequestScoped);
        bp.constructor(f!(crate::response_ref), Lifecycle::RequestScoped);
        bp.constructor(f!(crate::request_head_ref), Lifecycle::RequestScoped);
        bp.constructor(f!(crate::allowed_methods_ref), Lifecycle::RequestScoped);
        bp.constructor(f!(crate::raw_incoming_body_ref), Lifecycle::RequestScoped);
        bp.constructor(
            f!(crate::matched_path_pattern_ref),
            Lifecycle::RequestScoped,
        );
        bp.constructor(f!(crate::raw_path_params_ref), Lifecycle::RequestScoped);
        bp
    });
    bp
}
