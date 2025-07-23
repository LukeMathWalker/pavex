//! px:user_constructor
use pavex::methods;

pub enum User {
    Anonymous,
    Authenticated(AuthenticatedUser),
}

pub struct AuthenticatedUser {
    pub id: u64,
}

#[methods] // px::ann:1
impl User {
    #[request_scoped] // px::hl
    pub fn extract() -> Self {
        // Business logic goes here
        todo!() // px::skip
    }
}
