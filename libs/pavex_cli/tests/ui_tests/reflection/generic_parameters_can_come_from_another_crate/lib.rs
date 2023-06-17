use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;
use pavex::response::IntoResponse;
use pavex::response::Response;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET, "/home", f!(crate::handler));
    bp
}

// A locally-definited type
pub struct BodyType {
    pub name: String,
    pub age: u8,
}

impl http_body::Body for BodyType {
    type Data = bytes::Bytes;
    type Error = pavex::Error;

    fn poll_data(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<Self::Data, Self::Error>>> {
        todo!()
    }

    fn poll_trailers(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<Option<pavex::http::HeaderMap>, Self::Error>> {
        todo!()
    }
}

// The `Response` type comes from `pavex` but the body
// type is defined in this crate.
pub fn handler() -> Response<BodyType> {
    todo!()
}
