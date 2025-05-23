use pavex::blueprint::{from, router::GET, Blueprint};
use pavex::f;
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

        #[pavex::methods]
        impl GenericError<String> {
            #[pavex::error_handler]
            pub fn handle(
                #[px(error_ref)] &self,
                _b: &super::annotated::B,
            ) -> pavex::response::Response {
                todo!()
            }
        }
    }

    /// To be imported implicitly.
    pub mod annotated {
        pub struct B;

        #[pavex::request_scoped]
        pub fn b() -> Result<B, ErrorB> {
            todo!()
        }

        #[derive(Debug)]
        pub struct ErrorB;

        #[pavex::methods]
        impl ErrorB {
            #[pavex::error_handler]
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

#[pavex::methods]
impl CustomError {
    #[pavex::error_handler]
    pub fn into_response(&self) -> Response {
        todo!()
    }
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from!(crate));
    bp.request_scoped(f!(crate::constructor::raw::a));
    bp.route(GET, "/", f!(crate::handler));
    bp
}
