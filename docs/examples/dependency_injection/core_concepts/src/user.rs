use pavex::{methods, request::RequestHead};

pub enum User {
    Anonymous,
    Authenticated(AuthenticatedUser),
}

pub struct AuthenticatedUser {
    pub id: u64,
}

// px:user_constructor2:start
#[methods]
impl User {
    #[request_scoped]
    pub fn extract(head: &RequestHead /* px::ann:1 */) -> Self {
        todo!() // px::skip
    }
}
// px:user_constructor2:end
