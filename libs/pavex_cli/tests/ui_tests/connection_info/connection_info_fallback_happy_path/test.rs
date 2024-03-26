use std::future::IntoFuture;
use std::net::TcpListener;

use http_body_util::{BodyExt, Empty};
use hyper::body::Bytes;
use hyper::client::conn::http1::handshake;
use hyper::Request;
use hyper_util::rt::TokioIo;
use tokio::io::AsyncWriteExt;

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
    let addr = format!("localhost:{port}");
    let stream = tokio::net::TcpStream::connect(addr)
        .await
        .expect("Failed to connect TCP stream");
    let local_addr = stream.local_addr().expect("Failed to get local address");
    let io = TokioIo::new(stream);
    let (mut sender, conn) = handshake(io).await.expect("TCP handshake failed");
    tokio::task::spawn(async move { conn.await.expect("TCP connection failed") });

    let req = Request::builder()
        .uri("/")
        .body(Empty::<Bytes>::new())
        .expect("Failed to construct request");

    let mut res = sender
        .send_request(req)
        .await
        .expect("Failed to send request");

    let mut body = Vec::new();
    while let Some(next) = res.frame().await {
        let frame = next.expect("Failed to get frame");
        if let Some(chunk) = frame.data_ref() {
            body.write_all(&chunk).await.expect("Failed to write chunk");
        }
    }
    let body = String::from_utf8(body).expect("Body is not UTF8");

    assert_eq!(format!("{local_addr}"), body);
}
