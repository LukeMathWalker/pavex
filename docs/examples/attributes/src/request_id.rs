//! px:trait_method
use pavex::methods;

pub struct RequestId(uuid::Uuid);

#[methods] // px::ann:1
impl Default for RequestId {
    #[request_scoped]
    fn default() -> Self {
        Self(uuid::Uuid::now_v7())
    }
}
