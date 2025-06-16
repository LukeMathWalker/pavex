use pavex::blueprint::{from, Blueprint};

pub mod owned {
    use pavex::connection::ConnectionInfo;
    use pavex::request::{
        body::RawIncomingBody,
        path::{MatchedPathPattern, RawPathParams},
        RequestHead,
    };
    use pavex::router::AllowedMethods;

    #[pavex::request_scoped]
    pub fn error() -> pavex::Error {
        todo!()
    }

    #[pavex::request_scoped]
    pub fn response() -> pavex::response::Response {
        todo!()
    }

    #[pavex::request_scoped]
    pub fn request_head() -> RequestHead {
        todo!()
    }

    #[pavex::request_scoped]
    pub fn raw_incoming_body() -> RawIncomingBody {
        todo!()
    }

    #[pavex::request_scoped]
    pub fn allowed_methods() -> AllowedMethods {
        todo!()
    }

    #[pavex::request_scoped]
    pub fn matched_path_pattern() -> MatchedPathPattern {
        todo!()
    }

    #[pavex::request_scoped]
    pub fn raw_path_params(_v: &RequestHead) -> RawPathParams {
        todo!()
    }

    #[pavex::request_scoped]
    pub fn connection_info() -> ConnectionInfo {
        todo!()
    }
}

pub mod ref_ {
    use pavex::request::{
        body::RawIncomingBody,
        path::{MatchedPathPattern, RawPathParams},
        RequestHead,
    };
    use pavex::router::AllowedMethods;

    #[pavex::request_scoped]
    pub fn error_ref(_v: &RequestHead) -> &pavex::Error {
        todo!()
    }

    #[pavex::request_scoped]
    pub fn response_ref(_v: &RequestHead) -> &pavex::response::Response {
        todo!()
    }

    #[pavex::request_scoped]
    pub fn request_head_ref(_v: &AllowedMethods) -> &RequestHead {
        todo!()
    }

    #[pavex::request_scoped]
    pub fn raw_incoming_body_ref(_v: &AllowedMethods) -> &RawIncomingBody {
        todo!()
    }

    #[pavex::request_scoped]
    pub fn allowed_methods_ref(_v: &RequestHead) -> &AllowedMethods {
        todo!()
    }

    #[pavex::request_scoped]
    pub fn matched_path_pattern_ref(_v: &RequestHead) -> &MatchedPathPattern {
        todo!()
    }

    #[pavex::request_scoped]
    pub fn raw_path_params_ref(_v: &RequestHead) -> &RawPathParams {
        todo!()
    }
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.nest({
        let mut bp = Blueprint::new();
        bp.import(from![crate::owned]);
        bp
    });
    bp.nest({
        let mut bp = Blueprint::new();
        bp.import(from![crate::ref_]);
        bp
    });
    bp
}
