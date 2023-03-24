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
pub async fn build_application_state(v0: app::Config) -> crate::ApplicationState {
    let v1 = app::http_client(v0);
    crate::ApplicationState { s0: v1 }
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
    let matched_route = match server_state.router.at(request.uri().path()) {
        Ok(m) => m,
        Err(_) => {
            return pavex_runtime::response::Response::builder()
                .status(pavex_runtime::http::StatusCode::NOT_FOUND)
                .body(pavex_runtime::body::boxed(hyper::body::Body::empty()))
                .unwrap();
        }
    };
    let route_id = matched_route.value;
    #[allow(unused)]
    let url_params: pavex_runtime::extract::route::RawRouteParams<'_, '_> = matched_route
        .params
        .into();
    match route_id {
        0u32 => {
            match request.method() {
                &pavex_runtime::http::Method::GET => {
                    route_handler_0(server_state.application_state.s0.clone(), request)
                        .await
                }
                _ => {
                    pavex_runtime::response::Response::builder()
                        .status(pavex_runtime::http::StatusCode::METHOD_NOT_ALLOWED)
                        .header(pavex_runtime::http::header::ALLOW, "GET")
                        .body(pavex_runtime::body::boxed(hyper::body::Body::empty()))
                        .unwrap()
                }
            }
        }
        _ => {
            pavex_runtime::response::Response::builder()
                .status(pavex_runtime::http::StatusCode::NOT_FOUND)
                .body(pavex_runtime::body::boxed(hyper::body::Body::empty()))
                .unwrap()
        }
    }
}
pub async fn route_handler_0(
    v0: app::HttpClient,
    v1: http::Request<hyper::Body>,
) -> http::Response<
    http_body::combinators::BoxBody<bytes::Bytes, pavex_runtime::Error>,
> {
    let v2 = app::extract_path(v1).await;
    match v2 {
        Ok(v3) => {
            let v5 = {
                let v4 = app::logger();
                app::stream_file(v3, v4, v0)
            };
            <http::Response<
                http_body::combinators::BoxBody<bytes::Bytes, pavex_runtime::Error>,
            > as pavex_runtime::response::IntoResponse>::into_response(v5)
        }
        Err(v3) => {
            let v5 = {
                let v4 = app::logger();
                app::handle_extract_path_error(&v3, v4)
            };
            <http::Response<
                http_body::combinators::BoxBody<bytes::Bytes, pavex_runtime::Error>,
            > as pavex_runtime::response::IntoResponse>::into_response(v5)
        }
    }
}