use std::net::TcpListener;

use application::{build_application_state, run};

#[tokio::test]
async fn path_extraction_works() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to listen on a random port");
    let port = listener
        .local_addr()
        .expect("Failed to get local address")
        .port();
    let server =
        pavex_runtime::hyper::Server::from_tcp(listener).expect("Failed to create a hyper server");
    let application_state = build_application_state().await;
    tokio::task::spawn(async move {
        run(server, application_state)
            .await
            .expect("Failed to launch server");
    });
    let _response = reqwest::get(&format!("http://localhost:{}/home/123", port))
        .await
        .expect("Failed to make request")
        .error_for_status()
        .expect("Failed to get successful response");
}
