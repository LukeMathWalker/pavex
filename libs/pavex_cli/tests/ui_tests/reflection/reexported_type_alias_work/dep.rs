#[doc(inline)]
pub use api::engine;
#[doc(inline)]
pub use api::Surreal;

mod api {
    pub struct Surreal<T>(T);

    pub mod engine {
        pub struct Any;
    }
}
