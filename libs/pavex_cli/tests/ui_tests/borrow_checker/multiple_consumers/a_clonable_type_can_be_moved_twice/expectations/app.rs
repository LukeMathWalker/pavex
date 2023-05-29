//! Do NOT edit this code.
//! It was automatically generated by Pavex.
//! All manual edits will be lost next time the code is generated.
#[allow(unused_imports)]
use std as alloc;
struct ServerState {
    router: pavex_runtime::routing::Router<u32>,
    #[allow(dead_code)]
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
    router.insert("/home", 0u32)?;
    Ok(router)
}
async fn route_request(
    request: http::Request<pavex_runtime::hyper::body::Body>,
    server_state: std::sync::Arc<ServerState>,
) -> pavex_runtime::response::Response {
    #[allow(unused)]
    let (request_head, request_body) = request.into_parts();
    let request_head: pavex_runtime::request::RequestHead = request_head.into();
    let matched_route = match server_state.router.at(&request_head.uri.path()) {
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
            match &request_head.method {
                &pavex_runtime::http::Method::GET => route_handler_0().await,
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
pub async fn route_handler_0() -> http::Response<
    http_body::combinators::BoxBody<bytes::Bytes, pavex_runtime::Error>,
> {
    let v0 = app::a();
    let v1 = <app::A as core::clone::Clone>::clone(&v0);
    let v2 = app::c(v1);
    let v3 = app::b(v0);
    let v4 = app::handler(v3, v2);
    <http::Response<
        http_body::combinators::BoxBody<bytes::Bytes, pavex_runtime::Error>,
    > as pavex_runtime::response::IntoResponse>::into_response(v4)
}