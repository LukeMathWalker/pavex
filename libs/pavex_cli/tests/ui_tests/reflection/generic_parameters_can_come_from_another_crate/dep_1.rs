use pavex::response::IntoResponse;
use pavex::response::Response;

pub struct Custom<T>(T);

impl<T> IntoResponse for Custom<T> {
    fn into_response(self) -> Response {
        todo!()
    }
}
