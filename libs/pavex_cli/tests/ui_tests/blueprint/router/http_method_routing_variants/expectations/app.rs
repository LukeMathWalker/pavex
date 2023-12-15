//! Do NOT edit this code.
//! It was automatically generated by Pavex.
//! All manual edits will be lost next time the code is generated.
#[allow(unused_imports)]
use std as alloc;
struct ServerState {
    router: matchit::Router<u32>,
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
fn build_router() -> matchit::Router<u32> {
    let mut router = matchit::Router::new();
    router.insert("/any", 0u32).unwrap();
    router.insert("/any_w_extensions", 1u32).unwrap();
    router.insert("/connect", 2u32).unwrap();
    router.insert("/custom", 3u32).unwrap();
    router.insert("/delete", 4u32).unwrap();
    router.insert("/get", 5u32).unwrap();
    router.insert("/head", 6u32).unwrap();
    router.insert("/mixed", 7u32).unwrap();
    router.insert("/mixed_with_custom", 8u32).unwrap();
    router.insert("/options", 9u32).unwrap();
    router.insert("/patch", 10u32).unwrap();
    router.insert("/post", 11u32).unwrap();
    router.insert("/put", 12u32).unwrap();
    router.insert("/trace", 13u32).unwrap();
    router
}
async fn route_request(
    request: http::Request<hyper::body::Incoming>,
    server_state: std::sync::Arc<ServerState>,
) -> pavex::response::Response {
    #[allow(unused)]
    let (request_head, request_body) = request.into_parts();
    let request_body = pavex::request::body::RawIncomingBody::from(request_body);
    let request_head: pavex::request::RequestHead = request_head.into();
    let matched_route = match server_state.router.at(&request_head.uri.path()) {
        Ok(m) => m,
        Err(_) => {
            let allowed_methods: pavex::router::AllowedMethods = pavex::router::MethodAllowList::from_iter(
                    vec![],
                )
                .into();
            return route_14::handler(&allowed_methods).await;
        }
    };
    let route_id = matched_route.value;
    #[allow(unused)]
    let url_params: pavex::request::route::RawRouteParams<'_, '_> = matched_route
        .params
        .into();
    match route_id {
        0u32 => {
            match &request_head.method {
                &pavex::http::Method::CONNECT
                | &pavex::http::Method::DELETE
                | &pavex::http::Method::GET
                | &pavex::http::Method::HEAD
                | &pavex::http::Method::OPTIONS
                | &pavex::http::Method::PATCH
                | &pavex::http::Method::POST
                | &pavex::http::Method::PUT
                | &pavex::http::Method::TRACE => route_9::handler().await,
                _ => {
                    let allowed_methods: pavex::router::AllowedMethods = pavex::router::MethodAllowList::from_iter([
                            pavex::http::Method::CONNECT,
                            pavex::http::Method::DELETE,
                            pavex::http::Method::GET,
                            pavex::http::Method::HEAD,
                            pavex::http::Method::OPTIONS,
                            pavex::http::Method::PATCH,
                            pavex::http::Method::POST,
                            pavex::http::Method::PUT,
                            pavex::http::Method::TRACE,
                        ])
                        .into();
                    route_14::handler(&allowed_methods).await
                }
            }
        }
        1u32 => route_10::handler().await,
        2u32 => {
            match &request_head.method {
                &pavex::http::Method::CONNECT => route_0::handler().await,
                _ => {
                    let allowed_methods: pavex::router::AllowedMethods = pavex::router::MethodAllowList::from_iter([
                            pavex::http::Method::CONNECT,
                        ])
                        .into();
                    route_14::handler(&allowed_methods).await
                }
            }
        }
        3u32 => {
            match &request_head.method {
                s if s.as_str() == "CUSTOM" => route_12::handler().await,
                _ => {
                    let allowed_methods: pavex::router::AllowedMethods = pavex::router::MethodAllowList::from_iter([
                            pavex::http::Method::try_from("CUSTOM")
                                .expect("CUSTOM is not a valid (custom) HTTP method"),
                        ])
                        .into();
                    route_14::handler(&allowed_methods).await
                }
            }
        }
        4u32 => {
            match &request_head.method {
                &pavex::http::Method::DELETE => route_1::handler().await,
                _ => {
                    let allowed_methods: pavex::router::AllowedMethods = pavex::router::MethodAllowList::from_iter([
                            pavex::http::Method::DELETE,
                        ])
                        .into();
                    route_14::handler(&allowed_methods).await
                }
            }
        }
        5u32 => {
            match &request_head.method {
                &pavex::http::Method::GET => route_2::handler().await,
                _ => {
                    let allowed_methods: pavex::router::AllowedMethods = pavex::router::MethodAllowList::from_iter([
                            pavex::http::Method::GET,
                        ])
                        .into();
                    route_14::handler(&allowed_methods).await
                }
            }
        }
        6u32 => {
            match &request_head.method {
                &pavex::http::Method::HEAD => route_3::handler().await,
                _ => {
                    let allowed_methods: pavex::router::AllowedMethods = pavex::router::MethodAllowList::from_iter([
                            pavex::http::Method::HEAD,
                        ])
                        .into();
                    route_14::handler(&allowed_methods).await
                }
            }
        }
        7u32 => {
            match &request_head.method {
                &pavex::http::Method::PATCH | &pavex::http::Method::POST => {
                    route_11::handler().await
                }
                _ => {
                    let allowed_methods: pavex::router::AllowedMethods = pavex::router::MethodAllowList::from_iter([
                            pavex::http::Method::PATCH,
                            pavex::http::Method::POST,
                        ])
                        .into();
                    route_14::handler(&allowed_methods).await
                }
            }
        }
        8u32 => {
            match &request_head.method {
                &pavex::http::Method::GET => route_13::handler().await,
                s if s.as_str() == "CUSTOM" || s.as_str() == "HEY" => {
                    route_13::handler().await
                }
                _ => {
                    let allowed_methods: pavex::router::AllowedMethods = pavex::router::MethodAllowList::from_iter([
                            pavex::http::Method::try_from("CUSTOM")
                                .expect("CUSTOM is not a valid (custom) HTTP method"),
                            pavex::http::Method::GET,
                            pavex::http::Method::try_from("HEY")
                                .expect("HEY is not a valid (custom) HTTP method"),
                        ])
                        .into();
                    route_14::handler(&allowed_methods).await
                }
            }
        }
        9u32 => {
            match &request_head.method {
                &pavex::http::Method::OPTIONS => route_4::handler().await,
                _ => {
                    let allowed_methods: pavex::router::AllowedMethods = pavex::router::MethodAllowList::from_iter([
                            pavex::http::Method::OPTIONS,
                        ])
                        .into();
                    route_14::handler(&allowed_methods).await
                }
            }
        }
        10u32 => {
            match &request_head.method {
                &pavex::http::Method::PATCH => route_5::handler().await,
                _ => {
                    let allowed_methods: pavex::router::AllowedMethods = pavex::router::MethodAllowList::from_iter([
                            pavex::http::Method::PATCH,
                        ])
                        .into();
                    route_14::handler(&allowed_methods).await
                }
            }
        }
        11u32 => {
            match &request_head.method {
                &pavex::http::Method::POST => route_6::handler().await,
                _ => {
                    let allowed_methods: pavex::router::AllowedMethods = pavex::router::MethodAllowList::from_iter([
                            pavex::http::Method::POST,
                        ])
                        .into();
                    route_14::handler(&allowed_methods).await
                }
            }
        }
        12u32 => {
            match &request_head.method {
                &pavex::http::Method::PUT => route_7::handler().await,
                _ => {
                    let allowed_methods: pavex::router::AllowedMethods = pavex::router::MethodAllowList::from_iter([
                            pavex::http::Method::PUT,
                        ])
                        .into();
                    route_14::handler(&allowed_methods).await
                }
            }
        }
        13u32 => {
            match &request_head.method {
                &pavex::http::Method::TRACE => route_8::handler().await,
                _ => {
                    let allowed_methods: pavex::router::AllowedMethods = pavex::router::MethodAllowList::from_iter([
                            pavex::http::Method::TRACE,
                        ])
                        .into();
                    route_14::handler(&allowed_methods).await
                }
            }
        }
        i => unreachable!("Unknown route id: {}", i),
    }
}
pub mod route_0 {
    pub async fn handler() -> pavex::response::Response {
        let v0 = app::handler();
        <pavex::response::Response as pavex::response::IntoResponse>::into_response(v0)
    }
}
pub mod route_1 {
    pub async fn handler() -> pavex::response::Response {
        let v0 = app::handler();
        <pavex::response::Response as pavex::response::IntoResponse>::into_response(v0)
    }
}
pub mod route_2 {
    pub async fn handler() -> pavex::response::Response {
        let v0 = app::handler();
        <pavex::response::Response as pavex::response::IntoResponse>::into_response(v0)
    }
}
pub mod route_3 {
    pub async fn handler() -> pavex::response::Response {
        let v0 = app::handler();
        <pavex::response::Response as pavex::response::IntoResponse>::into_response(v0)
    }
}
pub mod route_4 {
    pub async fn handler() -> pavex::response::Response {
        let v0 = app::handler();
        <pavex::response::Response as pavex::response::IntoResponse>::into_response(v0)
    }
}
pub mod route_5 {
    pub async fn handler() -> pavex::response::Response {
        let v0 = app::handler();
        <pavex::response::Response as pavex::response::IntoResponse>::into_response(v0)
    }
}
pub mod route_6 {
    pub async fn handler() -> pavex::response::Response {
        let v0 = app::handler();
        <pavex::response::Response as pavex::response::IntoResponse>::into_response(v0)
    }
}
pub mod route_7 {
    pub async fn handler() -> pavex::response::Response {
        let v0 = app::handler();
        <pavex::response::Response as pavex::response::IntoResponse>::into_response(v0)
    }
}
pub mod route_8 {
    pub async fn handler() -> pavex::response::Response {
        let v0 = app::handler();
        <pavex::response::Response as pavex::response::IntoResponse>::into_response(v0)
    }
}
pub mod route_9 {
    pub async fn handler() -> pavex::response::Response {
        let v0 = app::handler();
        <pavex::response::Response as pavex::response::IntoResponse>::into_response(v0)
    }
}
pub mod route_10 {
    pub async fn handler() -> pavex::response::Response {
        let v0 = app::handler();
        <pavex::response::Response as pavex::response::IntoResponse>::into_response(v0)
    }
}
pub mod route_11 {
    pub async fn handler() -> pavex::response::Response {
        let v0 = app::handler();
        <pavex::response::Response as pavex::response::IntoResponse>::into_response(v0)
    }
}
pub mod route_12 {
    pub async fn handler() -> pavex::response::Response {
        let v0 = app::handler();
        <pavex::response::Response as pavex::response::IntoResponse>::into_response(v0)
    }
}
pub mod route_13 {
    pub async fn handler() -> pavex::response::Response {
        let v0 = app::handler();
        <pavex::response::Response as pavex::response::IntoResponse>::into_response(v0)
    }
}
pub mod route_14 {
    pub async fn handler(
        v0: &pavex::router::AllowedMethods,
    ) -> pavex::response::Response {
        let v1 = pavex::router::default_fallback(v0).await;
        <pavex::response::Response as pavex::response::IntoResponse>::into_response(v1)
    }
}