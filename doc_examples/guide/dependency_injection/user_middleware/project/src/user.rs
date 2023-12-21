pub enum User {
    Anonymous,
    Authenticated(AuthenticatedUser),
}

pub struct AuthenticatedUser {
    pub id: u64,
}

impl User {
    pub fn extract() -> Self {
        todo!() // Business logic goes here
    }
}