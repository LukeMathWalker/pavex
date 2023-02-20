//! Do NOT edit this code.
//! It was automatically generated by `pavex`.
//! All manual edits will be lost next time the code is generated.
use std as alloc;
struct ServerState {
    router: pavex_runtime::routing::Router<u32>,
    application_state: ApplicationState,
}
pub struct ApplicationState {}
pub async fn build_application_state() -> crate::ApplicationState {
    crate::ApplicationState {}
}
pub async fn run(
    server_builder: pavex_runtime::hyper::server::Builder<
        pavex_runtime::hyper::server::conn::AddrIncoming,
    >,
    application_state: ApplicationState,
) -> Result<(), pavex_runtime::Error> {
    let server_state = std::sync::Arc::new(ServerState {
        router: build_router().map_err(pavex_runtime::Error::new)?,
        application_state,
    });
    let make_service = pavex_runtime::hyper::service::make_service_fn(move |_| {
        let server_state = server_state.clone();
        async move {
            Ok::<
                _,
                pavex_runtime::hyper::Error,
            >(
                pavex_runtime::hyper::service::service_fn(move |request| {
                    let server_state = server_state.clone();
                    async move {
                        Ok::<
                            _,
                            pavex_runtime::hyper::Error,
                        >(route_request(request, server_state).await)
                    }
                }),
            )
        }
    });
    server_builder.serve(make_service).await.map_err(pavex_runtime::Error::new)
}
fn build_router() -> Result<
    pavex_runtime::routing::Router<u32>,
    pavex_runtime::routing::InsertError,
> {
    let mut router = pavex_runtime::routing::Router::new();
    router.insert("/any", 0u32)?;
    router.insert("/connect", 1u32)?;
    router.insert("/delete", 2u32)?;
    router.insert("/get", 3u32)?;
    router.insert("/head", 4u32)?;
    router.insert("/mixed", 5u32)?;
    router.insert("/options", 6u32)?;
    router.insert("/patch", 7u32)?;
    router.insert("/post", 8u32)?;
    router.insert("/put", 9u32)?;
    router.insert("/trace", 10u32)?;
    Ok(router)
}
async fn route_request(
    request: pavex_runtime::http::Request<pavex_runtime::hyper::body::Body>,
    server_state: std::sync::Arc<ServerState>,
) -> pavex_runtime::response::Response {
    let route_id = server_state
        .router
        .at(request.uri().path())
        .expect("Failed to match incoming request path");
    match route_id.value {
        0u32 => route_handler_0().await,
        1u32 => {
            match request.method() {
                &pavex_runtime::http::Method::CONNECT => route_handler_1().await,
                s => {
                    pavex_runtime::response::Response::builder()
                        .status(http::StatusCode::METHOD_NOT_ALLOWED)
                        .header(pavex_runtime::http::header::ALLOW, "CONNECT")
                        .body(pavex_runtime::body::boxed(hyper::body::Body::empty()))
                        .unwrap()
                }
            }
        }
        2u32 => {
            match request.method() {
                &pavex_runtime::http::Method::DELETE => route_handler_2().await,
                s => {
                    pavex_runtime::response::Response::builder()
                        .status(http::StatusCode::METHOD_NOT_ALLOWED)
                        .header(pavex_runtime::http::header::ALLOW, "DELETE")
                        .body(pavex_runtime::body::boxed(hyper::body::Body::empty()))
                        .unwrap()
                }
            }
        }
        3u32 => {
            match request.method() {
                &pavex_runtime::http::Method::GET => route_handler_3().await,
                s => {
                    pavex_runtime::response::Response::builder()
                        .status(http::StatusCode::METHOD_NOT_ALLOWED)
                        .header(pavex_runtime::http::header::ALLOW, "GET")
                        .body(pavex_runtime::body::boxed(hyper::body::Body::empty()))
                        .unwrap()
                }
            }
        }
        4u32 => {
            match request.method() {
                &pavex_runtime::http::Method::HEAD => route_handler_4().await,
                s => {
                    pavex_runtime::response::Response::builder()
                        .status(http::StatusCode::METHOD_NOT_ALLOWED)
                        .header(pavex_runtime::http::header::ALLOW, "HEAD")
                        .body(pavex_runtime::body::boxed(hyper::body::Body::empty()))
                        .unwrap()
                }
            }
        }
        5u32 => {
            match request.method() {
                &pavex_runtime::http::Method::PATCH => route_handler_5().await,
                &pavex_runtime::http::Method::POST => route_handler_5().await,
                s => {
                    pavex_runtime::response::Response::builder()
                        .status(http::StatusCode::METHOD_NOT_ALLOWED)
                        .header(pavex_runtime::http::header::ALLOW, "PATCH, POST")
                        .body(pavex_runtime::body::boxed(hyper::body::Body::empty()))
                        .unwrap()
                }
            }
        }
        6u32 => {
            match request.method() {
                &pavex_runtime::http::Method::OPTIONS => route_handler_6().await,
                s => {
                    pavex_runtime::response::Response::builder()
                        .status(http::StatusCode::METHOD_NOT_ALLOWED)
                        .header(pavex_runtime::http::header::ALLOW, "OPTIONS")
                        .body(pavex_runtime::body::boxed(hyper::body::Body::empty()))
                        .unwrap()
                }
            }
        }
        7u32 => {
            match request.method() {
                &pavex_runtime::http::Method::PATCH => route_handler_7().await,
                s => {
                    pavex_runtime::response::Response::builder()
                        .status(http::StatusCode::METHOD_NOT_ALLOWED)
                        .header(pavex_runtime::http::header::ALLOW, "PATCH")
                        .body(pavex_runtime::body::boxed(hyper::body::Body::empty()))
                        .unwrap()
                }
            }
        }
        8u32 => {
            match request.method() {
                &pavex_runtime::http::Method::POST => route_handler_8().await,
                s => {
                    pavex_runtime::response::Response::builder()
                        .status(http::StatusCode::METHOD_NOT_ALLOWED)
                        .header(pavex_runtime::http::header::ALLOW, "POST")
                        .body(pavex_runtime::body::boxed(hyper::body::Body::empty()))
                        .unwrap()
                }
            }
        }
        9u32 => {
            match request.method() {
                &pavex_runtime::http::Method::PUT => route_handler_9().await,
                s => {
                    pavex_runtime::response::Response::builder()
                        .status(http::StatusCode::METHOD_NOT_ALLOWED)
                        .header(pavex_runtime::http::header::ALLOW, "PUT")
                        .body(pavex_runtime::body::boxed(hyper::body::Body::empty()))
                        .unwrap()
                }
            }
        }
        10u32 => {
            match request.method() {
                &pavex_runtime::http::Method::TRACE => route_handler_10().await,
                s => {
                    pavex_runtime::response::Response::builder()
                        .status(http::StatusCode::METHOD_NOT_ALLOWED)
                        .header(pavex_runtime::http::header::ALLOW, "TRACE")
                        .body(pavex_runtime::body::boxed(hyper::body::Body::empty()))
                        .unwrap()
                }
            }
        }
        _ => panic!("This is a bug, no route registered for a route id"),
    }
}
pub async fn route_handler_0() -> http::Response<
    http_body::combinators::BoxBody<bytes::Bytes, pavex_runtime::Error>,
> {
    let v0 = app::handler();
    <http::Response::<
        http_body::combinators::BoxBody::<bytes::Bytes, pavex_runtime::Error>,
    > as pavex_runtime::response::IntoResponse>::into_response(v0)
}
pub async fn route_handler_1() -> http::Response<
    http_body::combinators::BoxBody<bytes::Bytes, pavex_runtime::Error>,
> {
    let v0 = app::handler();
    <http::Response::<
        http_body::combinators::BoxBody::<bytes::Bytes, pavex_runtime::Error>,
    > as pavex_runtime::response::IntoResponse>::into_response(v0)
}
pub async fn route_handler_2() -> http::Response<
    http_body::combinators::BoxBody<bytes::Bytes, pavex_runtime::Error>,
> {
    let v0 = app::handler();
    <http::Response::<
        http_body::combinators::BoxBody::<bytes::Bytes, pavex_runtime::Error>,
    > as pavex_runtime::response::IntoResponse>::into_response(v0)
}
pub async fn route_handler_3() -> http::Response<
    http_body::combinators::BoxBody<bytes::Bytes, pavex_runtime::Error>,
> {
    let v0 = app::handler();
    <http::Response::<
        http_body::combinators::BoxBody::<bytes::Bytes, pavex_runtime::Error>,
    > as pavex_runtime::response::IntoResponse>::into_response(v0)
}
pub async fn route_handler_4() -> http::Response<
    http_body::combinators::BoxBody<bytes::Bytes, pavex_runtime::Error>,
> {
    let v0 = app::handler();
    <http::Response::<
        http_body::combinators::BoxBody::<bytes::Bytes, pavex_runtime::Error>,
    > as pavex_runtime::response::IntoResponse>::into_response(v0)
}
pub async fn route_handler_5() -> http::Response<
    http_body::combinators::BoxBody<bytes::Bytes, pavex_runtime::Error>,
> {
    let v0 = app::handler();
    <http::Response::<
        http_body::combinators::BoxBody::<bytes::Bytes, pavex_runtime::Error>,
    > as pavex_runtime::response::IntoResponse>::into_response(v0)
}
pub async fn route_handler_6() -> http::Response<
    http_body::combinators::BoxBody<bytes::Bytes, pavex_runtime::Error>,
> {
    let v0 = app::handler();
    <http::Response::<
        http_body::combinators::BoxBody::<bytes::Bytes, pavex_runtime::Error>,
    > as pavex_runtime::response::IntoResponse>::into_response(v0)
}
pub async fn route_handler_7() -> http::Response<
    http_body::combinators::BoxBody<bytes::Bytes, pavex_runtime::Error>,
> {
    let v0 = app::handler();
    <http::Response::<
        http_body::combinators::BoxBody::<bytes::Bytes, pavex_runtime::Error>,
    > as pavex_runtime::response::IntoResponse>::into_response(v0)
}
pub async fn route_handler_8() -> http::Response<
    http_body::combinators::BoxBody<bytes::Bytes, pavex_runtime::Error>,
> {
    let v0 = app::handler();
    <http::Response::<
        http_body::combinators::BoxBody::<bytes::Bytes, pavex_runtime::Error>,
    > as pavex_runtime::response::IntoResponse>::into_response(v0)
}
pub async fn route_handler_9() -> http::Response<
    http_body::combinators::BoxBody<bytes::Bytes, pavex_runtime::Error>,
> {
    let v0 = app::handler();
    <http::Response::<
        http_body::combinators::BoxBody::<bytes::Bytes, pavex_runtime::Error>,
    > as pavex_runtime::response::IntoResponse>::into_response(v0)
}
pub async fn route_handler_10() -> http::Response<
    http_body::combinators::BoxBody<bytes::Bytes, pavex_runtime::Error>,
> {
    let v0 = app::handler();
    <http::Response::<
        http_body::combinators::BoxBody::<bytes::Bytes, pavex_runtime::Error>,
    > as pavex_runtime::response::IntoResponse>::into_response(v0)
}