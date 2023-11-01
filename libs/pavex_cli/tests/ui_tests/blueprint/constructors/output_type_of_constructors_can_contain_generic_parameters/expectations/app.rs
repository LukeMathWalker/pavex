//! Do NOT edit this code.
//! It was automatically generated by Pavex.
//! All manual edits will be lost next time the code is generated.
#[allow(unused_imports)]
use std as alloc;
struct ServerState {
    router: pavex::routing::Router<u32>,
    #[allow(dead_code)]
    application_state: ApplicationState,
}
pub struct ApplicationState {}
pub async fn build_application_state() -> crate::ApplicationState {
    crate::ApplicationState {}
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
                &pavex::http::Method::GET => route_0::handler().await,
                _ => route_1::handler().await,
            }
        }
        _ => pavex::response::Response::not_found().box_body(),
    }
}
pub mod route_0 {
    pub async fn handler() -> pavex::response::Response {
        let v0 = app::fallible_with_generic_error2();
        let v1 = match v0 {
            Ok(ok) => ok,
            Err(v1) => {
                return {
                    let v2 = app::json();
                    let v3 = app::doubly_generic_error_handler(&v1, &v2);
                    <pavex::response::Response as pavex::response::IntoResponse>::into_response(
                        v3,
                    )
                };
            }
        };
        let v2 = app::fallible_with_generic_error();
        let v3 = match v2 {
            Ok(ok) => ok,
            Err(v3) => {
                return {
                    let v4 = app::generic_error_handler(&v3);
                    <pavex::response::Response as pavex::response::IntoResponse>::into_response(
                        v4,
                    )
                };
            }
        };
        let v4 = app::fallible_with_generic_error();
        let v5 = match v4 {
            Ok(ok) => ok,
            Err(v5) => {
                return {
                    let v6 = app::generic_error_handler(&v5);
                    <pavex::response::Response as pavex::response::IntoResponse>::into_response(
                        v6,
                    )
                };
            }
        };
        let v6 = app::fallible();
        let v7 = match v6 {
            Ok(ok) => ok,
            Err(v7) => {
                return {
                    let v8 = app::error_handler(&v7);
                    <pavex::response::Response as pavex::response::IntoResponse>::into_response(
                        v8,
                    )
                };
            }
        };
        let v8 = app::json();
        let v9 = app::json();
        let v10 = app::json();
        let v11 = app::handler(v8, v10, &v9, v7, v5, &v3, &v1);
        <pavex::response::Response as pavex::response::IntoResponse>::into_response(v11)
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