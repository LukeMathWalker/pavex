use pavex::response::Response;

pub fn greet_error_handler(e: &GreetError) -> Response {
    match e {
        GreetError::InvalidName => Response::bad_request().set_typed_body("Invalid name."),
        GreetError::DatabaseError => Response::internal_server_error()
            .set_typed_body("Something went wrong, please retry later."),
    }
}
