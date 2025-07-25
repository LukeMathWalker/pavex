//! px:di
use pavex::Response;
use pavex::methods;
// px::skip:start
pub struct AuthError {
    details: String,
}

pub struct OrganizationId(u64);
// px::skip:end

#[methods]
impl AuthError {
    #[error_handler]
    pub fn to_response(
        #[px(error_ref)] &self,          // px::ann:1
        organization_id: OrganizationId, // px::ann:2
    ) -> Response {
        Response::ok() // px::skip
    }
}
