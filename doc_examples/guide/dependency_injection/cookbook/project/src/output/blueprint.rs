use pavex::blueprint::constructor::Lifecycle;
use pavex::blueprint::Blueprint;
use pavex::f;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::output::parse), Lifecycle::RequestScoped);
    bp.constructor(
        f!(pavex::request::body::BufferedBody::extract),
        Lifecycle::RequestScoped,
    )
    .error_handler(f!(
        pavex::request::body::errors::ExtractBufferedBodyError::into_response
    ));
    bp.constructor(
        f!(<pavex::request::body::BodySizeLimit as std::default::Default>::default),
        Lifecycle::RequestScoped,
    );
    bp
}
