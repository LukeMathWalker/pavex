use std::net::SocketAddr;
use std::time::Duration;

use http::Request;
use hyper::body::Incoming;

use pavex::response::Response;
use pavex::server::{IncomingStream, Server, ServerConfiguration, ShutdownMode};

// A dummy handler for our server tests.
async fn test_handler(_request: Request<Incoming>, _state: ()) -> Response {
    Response::ok().box_body()
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

#[tokio::test]
async fn graceful() {
    async fn slow_handler(_req: Request<Incoming>, state: State) -> Response {
        // Signal that the connection has been established before starting to
        // sleep.
        state.started.send(()).await.unwrap();
        tokio::time::sleep(state.sleep).await;
        Response::ok().box_body()
    }

    #[derive(Clone)]
    struct State {
        sleep: Duration,
        started: tokio::sync::mpsc::Sender<()>,
    }

    let (incoming, addr) = test_incoming().await;
    let delay = Duration::from_millis(100);
    let (started_tx, mut started_rx) = tokio::sync::mpsc::channel(1);
    let state = State {
        sleep: delay,
        started: started_tx,
    };

    let server_handle = Server::new()
        .set_config(test_server_config())
        .listen(incoming)
        .serve(slow_handler, state);

    let url = format!("http://localhost:{}", addr.port());
    let get_response = async move {
        reqwest::get(url).await.unwrap().error_for_status().unwrap();
    };
    let get_response = tokio::task::spawn(get_response);

    // Wait for the connection to be established.
    started_rx.recv().await.unwrap();

    // Then start a graceful shutdown.
    let shutdown_future =
        tokio::task::spawn(server_handle.shutdown(ShutdownMode::Graceful { timeout: delay * 2 }));

    tokio::select! {
        v1 = get_response => {
            // The request must succeed!
            v1.unwrap();
        },
        _ = shutdown_future => {
            panic!("The server shutdown without waiting for an ongoing connection to complete within the allocated timeout")
        }
    }
}
