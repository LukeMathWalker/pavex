use pavex::blueprint::{from, router::GET, Blueprint};
use pavex::f;
use pavex::middleware::Processing;
use pavex::response::Response;

pub mod constructor {
    /// To be registered against the blueprint directly
    pub mod raw {
        pub struct A;

        pub fn a() -> Result<A, GenericError<String>> {
            todo!()
        }

        #[derive(Debug)]
        pub struct GenericError<T>(T);

        impl GenericError<String> {
            pub fn handle(&self, _b: &super::annotated::B) -> pavex::response::Response {
                todo!()
            }
        }
    }

    /// To be imported implicitly.
    pub mod annotated {
        pub struct B;

        #[pavex::request_scoped(error_handler = "self::ErrorB::into_response")]
        pub fn b() -> Result<B, ErrorB> {
            todo!()
        }

        #[derive(Debug)]
        pub struct ErrorB;

        impl ErrorB {
            pub fn into_response(&self) -> pavex::response::Response {
                todo!()
            }
        }

        pub struct Singleton;

        // No need to specify an error handler for singleton constructors.
        // The error is bubbled up by `ApplicationState::new`.
        #[pavex::singleton]
        pub fn singleton() -> Result<Singleton, SingletonError> {
            todo!()
        }

        #[derive(Debug, thiserror::Error)]
        #[error("The error message")]
        pub struct SingletonError;
    }
}

pub fn handler(
    _a: &constructor::raw::A,
    _b: &constructor::annotated::B,
    _singleton: &constructor::annotated::Singleton,
) -> Result<Response, CustomError> {
    todo!()
}

#[derive(Debug)]
pub struct CustomError;

impl CustomError {
    pub fn into_response(&self) -> Response {
        todo!()
    }

    pub fn into_response_override(&self) -> Response {
        todo!()
    }
}

pub fn post(_response: Response) -> Result<Response, CustomError> {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from!(crate));
    bp.request_scoped(f!(crate::constructor::raw::a))
        .error_handler(f!(crate::constructor::raw::GenericError::<
            std::string::String,
        >::handle));

    bp.post_process(f!(crate::post))
        .error_handler(f!(crate::CustomError::into_response));
    bp.route(GET, "/", f!(crate::handler))
        .error_handler(f!(crate::CustomError::into_response));
    bp
}
