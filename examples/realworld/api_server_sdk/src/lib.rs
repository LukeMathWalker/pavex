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
    s0: sqlx_core::driver_prelude::pool::Pool<sqlx_postgres::Postgres>,
}
#[derive(Debug)]
pub enum ApplicationStateError {
    CreateDbPool(sqlx_core::Error),
}
pub async fn build_application_state(
    v0: sqlx_postgres::PgConnectOptions,
) -> Result<crate::ApplicationState, crate::ApplicationStateError> {
    let v1 = conduit_core::routes::create_db_pool(v0).await;
    match v1 {
        Ok(v2) => {
            let v3 = crate::ApplicationState { s0: v2 };
            core::result::Result::Ok(v3)
        }
        Err(v2) => {
            let v3 = crate::ApplicationStateError::CreateDbPool(v2);
            core::result::Result::Err(v3)
        }
    }
}
pub async fn run(
    server_builder: pavex::hyper::server::Builder<
        pavex::hyper::server::conn::AddrIncoming,
    >,
    application_state: ApplicationState,
) -> Result<(), pavex::Error> {
    let server_state = std::sync::Arc::new(ServerState {
        router: build_router().map_err(pavex::Error::new)?,
        application_state,
    });
    let make_service = pavex::hyper::service::make_service_fn(move |_| {
        let server_state = server_state.clone();
        async move {
            Ok::<
                _,
                pavex::hyper::Error,
            >(
                pavex::hyper::service::service_fn(move |request| {
                    let server_state = server_state.clone();
                    async move {
                        let response = route_request(request, server_state).await;
                        let response = pavex::hyper::Response::from(response);
                        Ok::<_, pavex::hyper::Error>(response)
                    }
                }),
            )
        }
    });
    server_builder.serve(make_service).await.map_err(pavex::Error::new)
}
fn build_router() -> Result<pavex::routing::Router<u32>, pavex::routing::InsertError> {
    let mut router = pavex::routing::Router::new();
    router.insert("/api/ping", 0u32)?;
    router.insert("/articles", 1u32)?;
    router.insert("/articles/:slug", 2u32)?;
    router.insert("/articles/:slug/comments", 3u32)?;
    router.insert("/articles/:slug/comments/:comment_id", 4u32)?;
    router.insert("/articles/:slug/favorite", 5u32)?;
    router.insert("/articles/feed", 6u32)?;
    router.insert("/profiles/:username", 7u32)?;
    router.insert("/profiles/:username/follow", 8u32)?;
    router.insert("/tags", 9u32)?;
    router.insert("/user", 10u32)?;
    router.insert("/users", 11u32)?;
    router.insert("/users/login", 12u32)?;
    Ok(router)
}
async fn route_request(
    request: http::Request<pavex::hyper::body::Body>,
    server_state: std::sync::Arc<ServerState>,
) -> pavex::response::Response {
    #[allow(unused)]
    let (request_head, request_body) = request.into_parts();
    let request_head: pavex::request::RequestHead = request_head.into();
    let matched_route = match server_state.router.at(&request_head.uri.path()) {
        Ok(m) => m,
        Err(_) => {
            return pavex::response::Response::not_found().box_body();
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
                &pavex::http::Method::GET => route_handler_0().await,
                _ => {
                    let header_value = pavex::http::HeaderValue::from_static("GET");
                    pavex::response::Response::method_not_allowed()
                        .insert_header(pavex::http::header::ALLOW, header_value)
                        .box_body()
                }
            }
        }
        1u32 => {
            match &request_head.method {
                &pavex::http::Method::GET => route_handler_1(&request_head).await,
                &pavex::http::Method::POST => {
                    route_handler_2(request_body, &request_head).await
                }
                _ => {
                    let header_value = pavex::http::HeaderValue::from_static(
                        "GET, POST",
                    );
                    pavex::response::Response::method_not_allowed()
                        .insert_header(pavex::http::header::ALLOW, header_value)
                        .box_body()
                }
            }
        }
        2u32 => {
            match &request_head.method {
                &pavex::http::Method::DELETE => route_handler_3(url_params).await,
                &pavex::http::Method::GET => route_handler_4(url_params).await,
                &pavex::http::Method::PUT => {
                    route_handler_5(request_body, url_params, &request_head).await
                }
                _ => {
                    let header_value = pavex::http::HeaderValue::from_static(
                        "DELETE, GET, PUT",
                    );
                    pavex::response::Response::method_not_allowed()
                        .insert_header(pavex::http::header::ALLOW, header_value)
                        .box_body()
                }
            }
        }
        3u32 => {
            match &request_head.method {
                &pavex::http::Method::GET => route_handler_6(url_params).await,
                &pavex::http::Method::POST => {
                    route_handler_7(request_body, url_params, &request_head).await
                }
                _ => {
                    let header_value = pavex::http::HeaderValue::from_static(
                        "GET, POST",
                    );
                    pavex::response::Response::method_not_allowed()
                        .insert_header(pavex::http::header::ALLOW, header_value)
                        .box_body()
                }
            }
        }
        4u32 => {
            match &request_head.method {
                &pavex::http::Method::DELETE => route_handler_8(url_params).await,
                _ => {
                    let header_value = pavex::http::HeaderValue::from_static("DELETE");
                    pavex::response::Response::method_not_allowed()
                        .insert_header(pavex::http::header::ALLOW, header_value)
                        .box_body()
                }
            }
        }
        5u32 => {
            match &request_head.method {
                &pavex::http::Method::DELETE => route_handler_9(url_params).await,
                &pavex::http::Method::POST => route_handler_10(url_params).await,
                _ => {
                    let header_value = pavex::http::HeaderValue::from_static(
                        "DELETE, POST",
                    );
                    pavex::response::Response::method_not_allowed()
                        .insert_header(pavex::http::header::ALLOW, header_value)
                        .box_body()
                }
            }
        }
        6u32 => {
            match &request_head.method {
                &pavex::http::Method::GET => route_handler_11(&request_head).await,
                _ => {
                    let header_value = pavex::http::HeaderValue::from_static("GET");
                    pavex::response::Response::method_not_allowed()
                        .insert_header(pavex::http::header::ALLOW, header_value)
                        .box_body()
                }
            }
        }
        7u32 => {
            match &request_head.method {
                &pavex::http::Method::GET => route_handler_12(url_params).await,
                _ => {
                    let header_value = pavex::http::HeaderValue::from_static("GET");
                    pavex::response::Response::method_not_allowed()
                        .insert_header(pavex::http::header::ALLOW, header_value)
                        .box_body()
                }
            }
        }
        8u32 => {
            match &request_head.method {
                &pavex::http::Method::DELETE => route_handler_13(url_params).await,
                &pavex::http::Method::POST => route_handler_14(url_params).await,
                _ => {
                    let header_value = pavex::http::HeaderValue::from_static(
                        "DELETE, POST",
                    );
                    pavex::response::Response::method_not_allowed()
                        .insert_header(pavex::http::header::ALLOW, header_value)
                        .box_body()
                }
            }
        }
        9u32 => {
            match &request_head.method {
                &pavex::http::Method::GET => route_handler_15().await,
                _ => {
                    let header_value = pavex::http::HeaderValue::from_static("GET");
                    pavex::response::Response::method_not_allowed()
                        .insert_header(pavex::http::header::ALLOW, header_value)
                        .box_body()
                }
            }
        }
        10u32 => {
            match &request_head.method {
                &pavex::http::Method::GET => route_handler_16().await,
                &pavex::http::Method::PUT => {
                    route_handler_17(request_body, &request_head).await
                }
                _ => {
                    let header_value = pavex::http::HeaderValue::from_static("GET, PUT");
                    pavex::response::Response::method_not_allowed()
                        .insert_header(pavex::http::header::ALLOW, header_value)
                        .box_body()
                }
            }
        }
        11u32 => {
            match &request_head.method {
                &pavex::http::Method::POST => {
                    route_handler_18(request_body, &request_head).await
                }
                _ => {
                    let header_value = pavex::http::HeaderValue::from_static("POST");
                    pavex::response::Response::method_not_allowed()
                        .insert_header(pavex::http::header::ALLOW, header_value)
                        .box_body()
                }
            }
        }
        12u32 => {
            match &request_head.method {
                &pavex::http::Method::POST => {
                    route_handler_19(
                            &request_head,
                            request_body,
                            &server_state.application_state.s0,
                        )
                        .await
                }
                _ => {
                    let header_value = pavex::http::HeaderValue::from_static("POST");
                    pavex::response::Response::method_not_allowed()
                        .insert_header(pavex::http::header::ALLOW, header_value)
                        .box_body()
                }
            }
        }
        _ => pavex::response::Response::not_found().box_body(),
    }
}
pub async fn route_handler_0() -> pavex::response::Response {
    let v0 = conduit_core::routes::status::ping();
    <http::StatusCode as pavex::response::IntoResponse>::into_response(v0)
}
pub async fn route_handler_1(
    v0: &pavex::request::RequestHead,
) -> pavex::response::Response {
    let v1 = pavex::extract::query::QueryParams::extract(v0);
    match v1 {
        Ok(v2) => {
            let v3 = conduit_core::routes::articles::list_articles(v2);
            <http::StatusCode as pavex::response::IntoResponse>::into_response(v3)
        }
        Err(v2) => {
            let v3 = pavex::extract::query::errors::ExtractQueryParamsError::into_response(
                &v2,
            );
            <pavex::response::Response<
                http_body::Full<bytes::Bytes>,
            > as pavex::response::IntoResponse>::into_response(v3)
        }
    }
}
pub async fn route_handler_2(
    v0: hyper::Body,
    v1: &pavex::request::RequestHead,
) -> pavex::response::Response {
    let v2 = <pavex::extract::body::BodySizeLimit as std::default::Default>::default();
    let v3 = pavex::extract::body::BufferedBody::extract(v1, v0, v2).await;
    match v3 {
        Ok(v4) => {
            let v5 = pavex::extract::body::JsonBody::extract(v1, &v4);
            match v5 {
                Ok(v6) => {
                    let v7 = conduit_core::routes::articles::publish_article(v6);
                    <http::StatusCode as pavex::response::IntoResponse>::into_response(
                        v7,
                    )
                }
                Err(v6) => {
                    let v7 = pavex::extract::body::errors::ExtractJsonBodyError::into_response(
                        &v6,
                    );
                    <pavex::response::Response<
                        http_body::Full<bytes::Bytes>,
                    > as pavex::response::IntoResponse>::into_response(v7)
                }
            }
        }
        Err(v4) => {
            let v5 = pavex::extract::body::errors::ExtractBufferedBodyError::into_response(
                &v4,
            );
            <pavex::response::Response<
                http_body::Full<bytes::Bytes>,
            > as pavex::response::IntoResponse>::into_response(v5)
        }
    }
}
pub async fn route_handler_3(
    v0: pavex::extract::route::RawRouteParams<'_, '_>,
) -> pavex::response::Response {
    let v1 = pavex::extract::route::RouteParams::extract(v0);
    match v1 {
        Ok(v2) => {
            let v3 = conduit_core::routes::articles::delete_article(v2);
            <http::StatusCode as pavex::response::IntoResponse>::into_response(v3)
        }
        Err(v2) => {
            let v3 = pavex::extract::route::errors::ExtractRouteParamsError::into_response(
                &v2,
            );
            <pavex::response::Response<
                http_body::Full<bytes::Bytes>,
            > as pavex::response::IntoResponse>::into_response(v3)
        }
    }
}
pub async fn route_handler_4(
    v0: pavex::extract::route::RawRouteParams<'_, '_>,
) -> pavex::response::Response {
    let v1 = pavex::extract::route::RouteParams::extract(v0);
    match v1 {
        Ok(v2) => {
            let v3 = conduit_core::routes::articles::get_article(v2);
            <http::StatusCode as pavex::response::IntoResponse>::into_response(v3)
        }
        Err(v2) => {
            let v3 = pavex::extract::route::errors::ExtractRouteParamsError::into_response(
                &v2,
            );
            <pavex::response::Response<
                http_body::Full<bytes::Bytes>,
            > as pavex::response::IntoResponse>::into_response(v3)
        }
    }
}
pub async fn route_handler_5(
    v0: hyper::Body,
    v1: pavex::extract::route::RawRouteParams<'_, '_>,
    v2: &pavex::request::RequestHead,
) -> pavex::response::Response {
    let v3 = <pavex::extract::body::BodySizeLimit as std::default::Default>::default();
    let v4 = pavex::extract::body::BufferedBody::extract(v2, v0, v3).await;
    match v4 {
        Ok(v5) => {
            let v6 = pavex::extract::body::JsonBody::extract(v2, &v5);
            match v6 {
                Ok(v7) => {
                    let v8 = pavex::extract::route::RouteParams::extract(v1);
                    match v8 {
                        Ok(v9) => {
                            let v10 = conduit_core::routes::articles::update_article(
                                v9,
                                v7,
                            );
                            <http::StatusCode as pavex::response::IntoResponse>::into_response(
                                v10,
                            )
                        }
                        Err(v9) => {
                            let v10 = pavex::extract::route::errors::ExtractRouteParamsError::into_response(
                                &v9,
                            );
                            <pavex::response::Response<
                                http_body::Full<bytes::Bytes>,
                            > as pavex::response::IntoResponse>::into_response(v10)
                        }
                    }
                }
                Err(v7) => {
                    let v8 = pavex::extract::body::errors::ExtractJsonBodyError::into_response(
                        &v7,
                    );
                    <pavex::response::Response<
                        http_body::Full<bytes::Bytes>,
                    > as pavex::response::IntoResponse>::into_response(v8)
                }
            }
        }
        Err(v5) => {
            let v6 = pavex::extract::body::errors::ExtractBufferedBodyError::into_response(
                &v5,
            );
            <pavex::response::Response<
                http_body::Full<bytes::Bytes>,
            > as pavex::response::IntoResponse>::into_response(v6)
        }
    }
}
pub async fn route_handler_6(
    v0: pavex::extract::route::RawRouteParams<'_, '_>,
) -> pavex::response::Response {
    let v1 = pavex::extract::route::RouteParams::extract(v0);
    match v1 {
        Ok(v2) => {
            let v3 = conduit_core::routes::articles::list_comments(v2);
            <http::StatusCode as pavex::response::IntoResponse>::into_response(v3)
        }
        Err(v2) => {
            let v3 = pavex::extract::route::errors::ExtractRouteParamsError::into_response(
                &v2,
            );
            <pavex::response::Response<
                http_body::Full<bytes::Bytes>,
            > as pavex::response::IntoResponse>::into_response(v3)
        }
    }
}
pub async fn route_handler_7(
    v0: hyper::Body,
    v1: pavex::extract::route::RawRouteParams<'_, '_>,
    v2: &pavex::request::RequestHead,
) -> pavex::response::Response {
    let v3 = <pavex::extract::body::BodySizeLimit as std::default::Default>::default();
    let v4 = pavex::extract::body::BufferedBody::extract(v2, v0, v3).await;
    match v4 {
        Ok(v5) => {
            let v6 = pavex::extract::body::JsonBody::extract(v2, &v5);
            match v6 {
                Ok(v7) => {
                    let v8 = pavex::extract::route::RouteParams::extract(v1);
                    match v8 {
                        Ok(v9) => {
                            let v10 = conduit_core::routes::articles::publish_comment(
                                v9,
                                v7,
                            );
                            <http::StatusCode as pavex::response::IntoResponse>::into_response(
                                v10,
                            )
                        }
                        Err(v9) => {
                            let v10 = pavex::extract::route::errors::ExtractRouteParamsError::into_response(
                                &v9,
                            );
                            <pavex::response::Response<
                                http_body::Full<bytes::Bytes>,
                            > as pavex::response::IntoResponse>::into_response(v10)
                        }
                    }
                }
                Err(v7) => {
                    let v8 = pavex::extract::body::errors::ExtractJsonBodyError::into_response(
                        &v7,
                    );
                    <pavex::response::Response<
                        http_body::Full<bytes::Bytes>,
                    > as pavex::response::IntoResponse>::into_response(v8)
                }
            }
        }
        Err(v5) => {
            let v6 = pavex::extract::body::errors::ExtractBufferedBodyError::into_response(
                &v5,
            );
            <pavex::response::Response<
                http_body::Full<bytes::Bytes>,
            > as pavex::response::IntoResponse>::into_response(v6)
        }
    }
}
pub async fn route_handler_8(
    v0: pavex::extract::route::RawRouteParams<'_, '_>,
) -> pavex::response::Response {
    let v1 = pavex::extract::route::RouteParams::extract(v0);
    match v1 {
        Ok(v2) => {
            let v3 = conduit_core::routes::articles::delete_comment(v2);
            <http::StatusCode as pavex::response::IntoResponse>::into_response(v3)
        }
        Err(v2) => {
            let v3 = pavex::extract::route::errors::ExtractRouteParamsError::into_response(
                &v2,
            );
            <pavex::response::Response<
                http_body::Full<bytes::Bytes>,
            > as pavex::response::IntoResponse>::into_response(v3)
        }
    }
}
pub async fn route_handler_9(
    v0: pavex::extract::route::RawRouteParams<'_, '_>,
) -> pavex::response::Response {
    let v1 = pavex::extract::route::RouteParams::extract(v0);
    match v1 {
        Ok(v2) => {
            let v3 = conduit_core::routes::articles::unfavorite_article(v2);
            <http::StatusCode as pavex::response::IntoResponse>::into_response(v3)
        }
        Err(v2) => {
            let v3 = pavex::extract::route::errors::ExtractRouteParamsError::into_response(
                &v2,
            );
            <pavex::response::Response<
                http_body::Full<bytes::Bytes>,
            > as pavex::response::IntoResponse>::into_response(v3)
        }
    }
}
pub async fn route_handler_10(
    v0: pavex::extract::route::RawRouteParams<'_, '_>,
) -> pavex::response::Response {
    let v1 = pavex::extract::route::RouteParams::extract(v0);
    match v1 {
        Ok(v2) => {
            let v3 = conduit_core::routes::articles::favorite_article(v2);
            <http::StatusCode as pavex::response::IntoResponse>::into_response(v3)
        }
        Err(v2) => {
            let v3 = pavex::extract::route::errors::ExtractRouteParamsError::into_response(
                &v2,
            );
            <pavex::response::Response<
                http_body::Full<bytes::Bytes>,
            > as pavex::response::IntoResponse>::into_response(v3)
        }
    }
}
pub async fn route_handler_11(
    v0: &pavex::request::RequestHead,
) -> pavex::response::Response {
    let v1 = pavex::extract::query::QueryParams::extract(v0);
    match v1 {
        Ok(v2) => {
            let v3 = conduit_core::routes::articles::get_feed(v2);
            <http::StatusCode as pavex::response::IntoResponse>::into_response(v3)
        }
        Err(v2) => {
            let v3 = pavex::extract::query::errors::ExtractQueryParamsError::into_response(
                &v2,
            );
            <pavex::response::Response<
                http_body::Full<bytes::Bytes>,
            > as pavex::response::IntoResponse>::into_response(v3)
        }
    }
}
pub async fn route_handler_12(
    v0: pavex::extract::route::RawRouteParams<'_, '_>,
) -> pavex::response::Response {
    let v1 = pavex::extract::route::RouteParams::extract(v0);
    match v1 {
        Ok(v2) => {
            let v3 = conduit_core::routes::profiles::get_profile(v2);
            <http::StatusCode as pavex::response::IntoResponse>::into_response(v3)
        }
        Err(v2) => {
            let v3 = pavex::extract::route::errors::ExtractRouteParamsError::into_response(
                &v2,
            );
            <pavex::response::Response<
                http_body::Full<bytes::Bytes>,
            > as pavex::response::IntoResponse>::into_response(v3)
        }
    }
}
pub async fn route_handler_13(
    v0: pavex::extract::route::RawRouteParams<'_, '_>,
) -> pavex::response::Response {
    let v1 = pavex::extract::route::RouteParams::extract(v0);
    match v1 {
        Ok(v2) => {
            let v3 = conduit_core::routes::profiles::unfollow_profile(v2);
            <http::StatusCode as pavex::response::IntoResponse>::into_response(v3)
        }
        Err(v2) => {
            let v3 = pavex::extract::route::errors::ExtractRouteParamsError::into_response(
                &v2,
            );
            <pavex::response::Response<
                http_body::Full<bytes::Bytes>,
            > as pavex::response::IntoResponse>::into_response(v3)
        }
    }
}
pub async fn route_handler_14(
    v0: pavex::extract::route::RawRouteParams<'_, '_>,
) -> pavex::response::Response {
    let v1 = pavex::extract::route::RouteParams::extract(v0);
    match v1 {
        Ok(v2) => {
            let v3 = conduit_core::routes::profiles::follow_profile(v2);
            <http::StatusCode as pavex::response::IntoResponse>::into_response(v3)
        }
        Err(v2) => {
            let v3 = pavex::extract::route::errors::ExtractRouteParamsError::into_response(
                &v2,
            );
            <pavex::response::Response<
                http_body::Full<bytes::Bytes>,
            > as pavex::response::IntoResponse>::into_response(v3)
        }
    }
}
pub async fn route_handler_15() -> pavex::response::Response {
    let v0 = conduit_core::routes::tags::get_tags();
    <http::StatusCode as pavex::response::IntoResponse>::into_response(v0)
}
pub async fn route_handler_16() -> pavex::response::Response {
    let v0 = conduit_core::routes::users::get_user();
    <http::StatusCode as pavex::response::IntoResponse>::into_response(v0)
}
pub async fn route_handler_17(
    v0: hyper::Body,
    v1: &pavex::request::RequestHead,
) -> pavex::response::Response {
    let v2 = <pavex::extract::body::BodySizeLimit as std::default::Default>::default();
    let v3 = pavex::extract::body::BufferedBody::extract(v1, v0, v2).await;
    match v3 {
        Ok(v4) => {
            let v5 = pavex::extract::body::JsonBody::extract(v1, &v4);
            match v5 {
                Ok(v6) => {
                    let v7 = conduit_core::routes::users::update_user(v6);
                    <http::StatusCode as pavex::response::IntoResponse>::into_response(
                        v7,
                    )
                }
                Err(v6) => {
                    let v7 = pavex::extract::body::errors::ExtractJsonBodyError::into_response(
                        &v6,
                    );
                    <pavex::response::Response<
                        http_body::Full<bytes::Bytes>,
                    > as pavex::response::IntoResponse>::into_response(v7)
                }
            }
        }
        Err(v4) => {
            let v5 = pavex::extract::body::errors::ExtractBufferedBodyError::into_response(
                &v4,
            );
            <pavex::response::Response<
                http_body::Full<bytes::Bytes>,
            > as pavex::response::IntoResponse>::into_response(v5)
        }
    }
}
pub async fn route_handler_18(
    v0: hyper::Body,
    v1: &pavex::request::RequestHead,
) -> pavex::response::Response {
    let v2 = <pavex::extract::body::BodySizeLimit as std::default::Default>::default();
    let v3 = pavex::extract::body::BufferedBody::extract(v1, v0, v2).await;
    match v3 {
        Ok(v4) => {
            let v5 = pavex::extract::body::JsonBody::extract(v1, &v4);
            match v5 {
                Ok(v6) => {
                    let v7 = conduit_core::routes::users::signup(v6);
                    <http::StatusCode as pavex::response::IntoResponse>::into_response(
                        v7,
                    )
                }
                Err(v6) => {
                    let v7 = pavex::extract::body::errors::ExtractJsonBodyError::into_response(
                        &v6,
                    );
                    <pavex::response::Response<
                        http_body::Full<bytes::Bytes>,
                    > as pavex::response::IntoResponse>::into_response(v7)
                }
            }
        }
        Err(v4) => {
            let v5 = pavex::extract::body::errors::ExtractBufferedBodyError::into_response(
                &v4,
            );
            <pavex::response::Response<
                http_body::Full<bytes::Bytes>,
            > as pavex::response::IntoResponse>::into_response(v5)
        }
    }
}
pub async fn route_handler_19(
    v0: &pavex::request::RequestHead,
    v1: hyper::Body,
    v2: &sqlx_core::driver_prelude::pool::Pool<sqlx_postgres::Postgres>,
) -> pavex::response::Response {
    let v3 = <pavex::extract::body::BodySizeLimit as std::default::Default>::default();
    let v4 = pavex::extract::body::BufferedBody::extract(v0, v1, v3).await;
    match v4 {
        Ok(v5) => {
            let v6 = pavex::extract::body::JsonBody::extract(v0, &v5);
            match v6 {
                Ok(v7) => {
                    let v8 = conduit_core::routes::users::login(v7, v2).await;
                    <http::StatusCode as pavex::response::IntoResponse>::into_response(
                        v8,
                    )
                }
                Err(v7) => {
                    let v8 = pavex::extract::body::errors::ExtractJsonBodyError::into_response(
                        &v7,
                    );
                    <pavex::response::Response<
                        http_body::Full<bytes::Bytes>,
                    > as pavex::response::IntoResponse>::into_response(v8)
                }
            }
        }
        Err(v5) => {
            let v6 = pavex::extract::body::errors::ExtractBufferedBodyError::into_response(
                &v5,
            );
            <pavex::response::Response<
                http_body::Full<bytes::Bytes>,
            > as pavex::response::IntoResponse>::into_response(v6)
        }
    }
}
