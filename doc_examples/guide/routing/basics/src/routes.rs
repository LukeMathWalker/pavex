use pavex::response::Response;

pub fn greet() -> Response {
    Response::ok().set_typed_body("Hello, world!").box_body()
}
