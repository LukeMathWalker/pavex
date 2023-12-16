use pavex::response::Response;

pub async fn greet() -> Response {
    Response::ok().box_body()
}
