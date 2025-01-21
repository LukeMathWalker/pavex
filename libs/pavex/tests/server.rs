use std::net::SocketAddr;
use std::time::Duration;

use http::Request;
use hyper::body::Incoming;

use pavex::connection::ConnectionInfo;
use pavex::response::Response;
use pavex::server::{IncomingStream, Server, ServerConfiguration, ShutdownMode};

// A dummy handler for our server tests.
async fn test_handler(
    _request: Request<Incoming>,
    _connection_info: Option<ConnectionInfo>,
    _state: (),
) -> Response {
    Response::ok()
}

fn test_server_config() -> ServerConfiguration {
    ServerConfiguration::new().set_n_workers(1)
}

async fn test_incoming() -> (IncomingStream, SocketAddr) {
    let i = IncomingStream::bind("127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let addr = i.local_addr().unwrap();
    (i, addr)
}

#[tokio::test]
async fn listen() {
    let (incoming, addr) = test_incoming().await;
    Server::new()
        .set_config(test_server_config())
        .listen(incoming)
        .serve(test_handler, ());

    // Check that we can connect successfully
    std::net::TcpStream::connect(addr).unwrap();
}

#[tokio::test]
async fn bind() {
    let (_, addr) = test_incoming().await;
    Server::new()
        .set_config(test_server_config())
        .bind(addr)
        .await
        .unwrap()
        .serve(test_handler, ());

    // Check that we can connect successfully
    std::net::TcpStream::connect(addr).unwrap();
}

#[tokio::test]
async fn multi_bind() {
    let (_, addr1) = test_incoming().await;
    let (_, addr2) = test_incoming().await;
    let (incoming3, addr3) = test_incoming().await;
    Server::new()
        .set_config(test_server_config())
        .bind(addr1)
        .await
        .unwrap()
        .bind(addr2)
        .await
        .unwrap()
        .listen(incoming3)
        .serve(test_handler, ());

    // Check that we can connect successfully
    // to all addresses.
    std::net::TcpStream::connect(addr1).unwrap();
    std::net::TcpStream::connect(addr2).unwrap();
    std::net::TcpStream::connect(addr3).unwrap();
}

#[tokio::test]
async fn serve() {
    let (incoming, addr) = test_incoming().await;
    Server::new()
        .set_config(test_server_config())
        .listen(incoming)
        .serve(test_handler, ());

    // Check that we can connect successfully
    let url = format!("http://localhost:{}", addr.port());
    reqwest::get(url).await.unwrap().error_for_status().unwrap();
}

async fn slow_handler(
    _req: Request<Incoming>,
    _connection_info: Option<ConnectionInfo>,
    state: SlowHandlerState,
) -> Response {
    // Signal that the connection has been established before starting to
    // sleep.
    state.started.send(()).await.unwrap();
    tokio::time::sleep(state.sleep).await;
    Response::ok()
}

#[derive(Clone)]
struct SlowHandlerState {
    sleep: Duration,
    started: tokio::sync::mpsc::Sender<()>,
}

impl SlowHandlerState {
    fn new(delay: Duration) -> (tokio::sync::mpsc::Receiver<()>, Self) {
        let (started_tx, started_rx) = tokio::sync::mpsc::channel(1);
        let state = SlowHandlerState {
            sleep: delay,
            started: started_tx,
        };
        (started_rx, state)
    }
}

#[tokio::test]
async fn graceful() {
    let (incoming, addr) = test_incoming().await;
    let delay = Duration::from_millis(100);
    let (mut has_started, state) = SlowHandlerState::new(delay);

    let server_handle = Server::new()
        .set_config(test_server_config())
        .listen(incoming)
        .serve(slow_handler, state);

    let get_response = async move {
        let url = format!("http://localhost:{}", addr.port());
        reqwest::get(url)
            .await
            .unwrap()
            .error_for_status()
            .expect("The request failed");
    };
    let get_response = tokio::task::spawn(get_response);

    // Wait for the connection to be established.
    has_started.recv().await.unwrap();

    // Then start a graceful shutdown.
    let shutdown_future =
        tokio::task::spawn(server_handle.shutdown(ShutdownMode::Graceful { timeout: delay * 5 }));

    shutdown_future
        .await
        .expect("Shutdown didn't succeed as expected");

    get_response.await.expect("The server shutdown without waiting for an ongoing connection to complete within the allocated timeout");
}

#[tokio::test]
async fn forced() {
    let (incoming, addr) = test_incoming().await;
    let delay = Duration::from_millis(100);
    let (mut has_started, state) = SlowHandlerState::new(delay);

    let server_handle = Server::new()
        .set_config(test_server_config())
        .listen(incoming)
        .serve(slow_handler, state);

    let get_response = async move {
        let url = format!("http://localhost:{}", addr.port());
        reqwest::get(url).await.unwrap().error_for_status().unwrap();
    };
    let get_response = tokio::task::spawn(get_response);

    // Wait for the connection to be established.
    has_started.recv().await.unwrap();

    // Then start a forced shutdown.
    let shutdown_future = tokio::task::spawn(server_handle.shutdown(ShutdownMode::Forced));

    shutdown_future
        .await
        .expect("Shutdown didn't succeed as expected");

    assert!(
        get_response.await.unwrap_err().is_panic(),
        "The server was supposed to shutdown forcefully, but it waited for the ongoing request"
    );
}

#[tokio::test]
async fn graceful_but_too_fast() {
    let (incoming, addr) = test_incoming().await;
    let delay = Duration::from_millis(200);
    let (mut has_started, state) = SlowHandlerState::new(delay);

    let server_handle = Server::new()
        .set_config(test_server_config())
        .listen(incoming)
        .serve(slow_handler, state);

    let get_response = async move {
        let url = format!("http://localhost:{}", addr.port());
        reqwest::get(url).await.unwrap().error_for_status().unwrap();
    };
    let get_response = tokio::task::spawn(get_response);

    // Wait for the connection to be established.
    has_started.recv().await.unwrap();

    // Then start a graceful shutdown with a timeout that will **not** allow the request
    // handling to complete in time.
    let shutdown_future =
        tokio::task::spawn(server_handle.shutdown(ShutdownMode::Graceful { timeout: delay / 5 }));

    shutdown_future
        .await
        .expect("Shutdown didn't succeed as expected");

    // The server should have shutdown gracefully, but the request should have failed.
    assert!(
        get_response.await.unwrap_err().is_panic(),
        "The server was supposed to shutdown forcefully the slow request, but it waited instead"
    );
}
