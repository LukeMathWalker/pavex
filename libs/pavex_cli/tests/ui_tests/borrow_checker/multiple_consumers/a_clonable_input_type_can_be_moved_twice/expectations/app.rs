//! Do NOT edit this code.
//! It was automatically generated by Pavex.
//! All manual edits will be lost next time the code is generated.
#[allow(unused_imports)]
use std as alloc;
struct ServerState {
    router: pavex::routing::Router<u32>,
    application_state: ApplicationState,
}
pub struct ApplicationState {
    s0: app::A,
}
pub async fn build_application_state() -> crate::ApplicationState {
    let v0 = app::a();
    crate::ApplicationState { s0: v0 }
}
pub fn run(
    server_builder: pavex::server::Server,
    application_state: ApplicationState,
) -> pavex::server::ServerHandle {
    let server_state = std::sync::Arc::new(ServerState {
        router: build_router(),
        application_state,
    });
    server_builder.serve(route_request, server_state)
}
fn build_router() -> pavex::routing::Router<u32> {
    let mut router = pavex::routing::Router::new();
    router.insert("/home", 0u32).unwrap();
    router
}
async fn route_request(
    request: http::Request<pavex::hyper::body::Incoming>,
    server_state: std::sync::Arc<ServerState>,
) -> pavex::response::Response {
    #[allow(unused)]
    let (request_head, request_body) = request.into_parts();
    let request_head: pavex::request::RequestHead = request_head.into();
    let matched_route = match server_state.router.at(&request_head.uri.path()) {
        Ok(m) => m,
        Err(_) => {
            return route_1::handler().await;
        }
    };
    let route_id = matched_route.value;
    #[allow(unused)]
    let url_params: pavex::extract::route::RawRouteParams<'_, '_> = matched_route
        .params
        .into();
    match route_id {
        0u32 => {
            match &request_head.method {
                &pavex::http::Method::GET => {
                    route_0::handler(server_state.application_state.s0.clone()).await
                }
                _ => route_1::handler().await,
            }
        }
        _ => pavex::response::Response::not_found().box_body(),
    }
}
pub mod route_0 {
    pub async fn handler(v0: app::A) -> pavex::response::Response {
        let v1 = <app::A as core::clone::Clone>::clone(&v0);
        let v2 = app::c(v1);
        let v3 = app::b(v0);
        let v4 = app::handler(v3, v2);
        <pavex::response::Response as pavex::response::IntoResponse>::into_response(v4)
    }
}
pub mod route_1 {
    pub async fn handler() -> pavex::response::Response {
        let v0 = pavex::router::default_fallback().await;
        <pavex::response::Response<
            http_body_util::Empty<bytes::Bytes>,
        > as pavex::response::IntoResponse>::into_response(v0)
    }
}