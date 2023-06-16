use std::net::TcpListener;

use application::{build_application_state, run};

async fn spawn_test_server() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to listen on a random port");
    let port = listener
        .local_addr()
        .expect("Failed to get local address")
        .port();
    let server = pavex::hyper::Server::from_tcp(listener).expect("Failed to create a hyper server");
    let application_state = build_application_state().await;
    tokio::task::spawn(async move {
        run(server, application_state)
            .await
            .expect("Failed to launch server");
    });
    port
}

#[tokio::test]
async fn route_parameter_extraction_works() {
    let port = spawn_test_server().await;
    let response = reqwest::get(&format!("http://localhost:{}/home/123", port))
        .await
        .expect("Failed to make request")
        .error_for_status()
        .expect("Failed to get successful response");
    let text = response.text().await.expect("Failed to get response body");
    assert_eq!("123", text);
}

#[tokio::test]
async fn catch_all_extraction_works_on_a_single_segment() {
    let port = spawn_test_server().await;
    let response = reqwest::get(&format!("http://localhost:{}/town/123", port))
        .await
        .expect("Failed to make request")
        .error_for_status()
        .expect("Failed to get successful response");
    let text = response.text().await.expect("Failed to get response body");
    assert_eq!("123", text);
}

#[tokio::test]
async fn catch_all_extraction_works_on_a_multiple_segments() {
    let port = spawn_test_server().await;
    let response = reqwest::get(&format!(
        "http://localhost:{}/town/123/street/hello%20mate",
        port
    ))
    .await
    .expect("Failed to make request")
    .error_for_status()
    .expect("Failed to get successful response");
    let text = response.text().await.expect("Failed to get response body");
    assert_eq!("123/street/hello mate", text);
}

#[tokio::test]
async fn catch_all_match_cannot_be_empty() {
    let port = spawn_test_server().await;
    let response = reqwest::get(&format!("http://localhost:{}/town/", port))
        .await
        .expect("Failed to make request");
    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn route_parameter_has_the_wrong_type() {
    let port = spawn_test_server().await;
    let response = reqwest::get(&format!("http://localhost:{}/home/abc", port))
        .await
        .expect("Failed to make request");
    assert_eq!(response.status(), 400);
    let text = response.text().await.expect("Failed to get response body");
    assert_eq!(
        "Invalid URL.\n`home_id` is set to `abc`, which we can't parse as a `u32`",
        text
    );
}

#[tokio::test]
async fn route_parameter_is_invalid_ut8() {
    let port = spawn_test_server().await;
    let response = reqwest::get(&format!("http://localhost:{}/home/%DE~%C7%1FY", port))
        .await
        .expect("Failed to make request");
    assert_eq!(response.status(), 400);
    let text = response.text().await.expect("Failed to get response body");
    assert_eq!(
        "Invalid URL.\n`%DE~%C7%1FY` cannot be used as `home_id` since it is not a well-formed UTF8 string when percent-decoded",
        text
    );
}

#[tokio::test]
async fn route_parameter_is_invalid_type() {
    let port = spawn_test_server().await;
    let response = reqwest::get(&format!("http://localhost:{}/home/123/room/123", port))
        .await
        .expect("Failed to make request");
    // This is a programmer error, so we expect a 500 and an opaque error message.
    assert_eq!(response.status(), 500);
    let text = response.text().await.expect("Failed to get response body");
    assert_eq!(
        "Something went wrong when trying to process the request",
        text
    );
}

#[tokio::test]
async fn route_parameters_cannot_be_empty() {
    let port = spawn_test_server().await;
    // This does not match `/home/:home_id` because the `home_id` parameter is empty.
    // There are no handlers registered for `/home` so this should return a 404.
    let response = reqwest::get(&format!("http://localhost:{}/home", port))
        .await
        .expect("Failed to make request");
    assert_eq!(response.status(), 404);
}
