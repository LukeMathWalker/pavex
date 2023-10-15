use std::net::SocketAddr;

use http::Request;
use hyper::body::Incoming;

use pavex::response::Response;
use pavex::server::{IncomingStream, Server, ServerConfiguration};

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
