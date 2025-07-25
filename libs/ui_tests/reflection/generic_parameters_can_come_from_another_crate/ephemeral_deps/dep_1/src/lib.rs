use pavex::IntoResponse;
use pavex::Response;

pub struct Custom<T>(T);

impl<T> IntoResponse for Custom<T> {
    fn into_response(self) -> Response {
        todo!()
    }
}
