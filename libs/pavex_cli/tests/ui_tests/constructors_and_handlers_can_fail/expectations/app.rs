//! Do NOT edit this code.
//! It was automatically generated by `pavex`.
//! All manual edits will be lost next time the code is generated.
use std as alloc;
struct ServerState {
    router: pavex_runtime::routing::Router<u32>,
    application_state: ApplicationState,
}
pub struct ApplicationState {
    s0: app::HttpClient,
}
#[derive(Debug)]
pub enum ApplicationStateError {
    HttpClient(app::HttpClientError),
}
pub async fn build_application_state(
    v0: app::Config,
) -> Result<crate::ApplicationState, crate::ApplicationStateError> {
    let v1 = app::http_client(v0);
    match v1 {
        Ok(v2) => {
            let v3 = crate::ApplicationState { s0: v2 };
            core::result::Result::Ok(v3)
        }
        Err(v2) => {
            let v3 = crate::ApplicationStateError::HttpClient(v2);
            core::result::Result::Err(v3)
        }
    }
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
    router.insert("/home", 0u32)?;
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
        0u32 => {
            match request.method() {
                &pavex_runtime::http::Method::GET => {
                    route_handler_0(server_state.application_state.s0.clone(), request)
                        .await
                }
                s => panic!("This is a bug, no handler registered for `{s}` method"),
            }
        }
        _ => panic!("This is a bug, no route registered for a route id"),
    }
}
pub async fn route_handler_0(
    v0: app::HttpClient,
    v1: http::Request<hyper::Body>,
) -> http::Response<
    http_body::combinators::BoxBody<bytes::Bytes, pavex_runtime::Error>,
> {
    match app::logger() {
        Ok(v2) => {
            let v3 = app::extract_path(v1);
            match v3 {
                Ok(v4) => {
                    let v5 = app::request_handler(v4, v2, v0);
                    match v5 {
                        Ok(v6) => {
                            <http::Response::<
                                http_body::combinators::BoxBody::<
                                    bytes::Bytes,
                                    pavex_runtime::Error,
                                >,
                            > as pavex_runtime::response::IntoResponse>::into_response(
                                v6,
                            )
                        }
                        Err(v6) => {
                            let v7 = app::handle_handler_error(&v6);
                            <http::Response::<
                                http_body::combinators::BoxBody::<
                                    bytes::Bytes,
                                    pavex_runtime::Error,
                                >,
                            > as pavex_runtime::response::IntoResponse>::into_response(
                                v7,
                            )
                        }
                    }
                }
                Err(v4) => {
                    match app::logger() {
                        Ok(v5) => {
                            let v6 = app::handle_extract_path_error(&v4, v5);
                            <http::Response::<
                                http_body::combinators::BoxBody::<
                                    bytes::Bytes,
                                    pavex_runtime::Error,
                                >,
                            > as pavex_runtime::response::IntoResponse>::into_response(
                                v6,
                            )
                        }
                        Err(v5) => {
                            let v6 = app::handle_logger_error(&v5);
                            <http::Response::<
                                http_body::combinators::BoxBody::<
                                    bytes::Bytes,
                                    pavex_runtime::Error,
                                >,
                            > as pavex_runtime::response::IntoResponse>::into_response(
                                v6,
                            )
                        }
                    }
                }
            }
        }
        Err(v2) => {
            let v3 = app::handle_logger_error(&v2);
            <http::Response::<
                http_body::combinators::BoxBody::<bytes::Bytes, pavex_runtime::Error>,
            > as pavex_runtime::response::IntoResponse>::into_response(v3)
        }
    }
}