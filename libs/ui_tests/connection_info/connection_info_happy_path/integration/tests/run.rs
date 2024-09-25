use std::future::IntoFuture;
use std::net::TcpListener;

use application::{build_application_state, run};

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
async fn connection_info_extraction_works() {
    let port = spawn_test_server().await;
    let response = reqwest::get(&format!("http://localhost:{port}"))
        .await
        .expect("Failed to make request")
        .error_for_status()
        .expect("Failed to get successful response");
    let text = response.text().await.expect("Failed to get response body");
    assert_eq!(format!("Success"), text);
}
