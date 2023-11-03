use std::future::IntoFuture;
use std::net::TcpListener;

use application::{build_application_state, run};
use pavex::response::Response;

async fn spawn_test_server() -> u16 {
    static TELEMETRY: std::sync::Once = std::sync::Once::new();
    TELEMETRY.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_span_events(tracing_subscriber::fmt::format::FmtSpan::FULL)
            .init();
    });

    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to listen on a random port");
    let port = listener
        .local_addr()
        .expect("Failed to get local address")
        .port();
    let incoming_stream: pavex::server::IncomingStream =
        listener.try_into().expect("Failed to convert listener");
    let server = pavex::server::Server::new().listen(incoming_stream);
    let application_state = build_application_state().await;
    tokio::task::spawn(run(server, application_state).into_future());
    port
}

#[tokio::test]
async fn same_path_different_method() {
    let port = spawn_test_server().await;
    let response = reqwest::post(&format!("http://localhost:{}/users/id", port))
        .await
        .expect("Failed to make request");
    assert_eq!(StatusCode::Teapot, response.status());
}
