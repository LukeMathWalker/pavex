//! Do NOT edit this code.
//! It was automatically generated by Pavex.
//! All manual edits will be lost next time the code is generated.
extern crate alloc;
struct ServerState {
    router: Router,
    #[allow(dead_code)]
    application_state: ApplicationState,
}
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ApplicationConfig {}
pub struct ApplicationState {}
impl ApplicationState {
    pub async fn new(
        _app_config: crate::ApplicationConfig,
    ) -> Result<crate::ApplicationState, crate::ApplicationStateError> {
        Ok(Self::_new().await)
    }
    async fn _new() -> crate::ApplicationState {
        crate::ApplicationState {}
    }
}
#[derive(Debug, thiserror::Error)]
pub enum ApplicationStateError {}
pub fn run(
    server_builder: pavex::server::Server,
    application_state: ApplicationState,
) -> pavex::server::ServerHandle {
    async fn handler(
        request: http::Request<hyper::body::Incoming>,
        connection_info: Option<pavex::connection::ConnectionInfo>,
        server_state: std::sync::Arc<ServerState>,
    ) -> pavex::Response {
        let (router, state) = (&server_state.router, &server_state.application_state);
        router.route(request, connection_info, state).await
    }
    let router = Router::new();
    let server_state = std::sync::Arc::new(ServerState {
        router,
        application_state,
    });
    server_builder.serve(handler, server_state)
}
struct Router {
    router: matchit::Router<u32>,
}
impl Router {
    /// Create a new router instance.
    ///
    /// This method is invoked once, when the server starts.
    pub fn new() -> Self {
        Self { router: Self::router() }
    }
    fn router() -> matchit::Router<u32> {
        let mut router = matchit::Router::new();
        router.insert("/nested{*catch_all}", 0u32).unwrap();
        router
    }
    pub async fn route(
        &self,
        request: http::Request<hyper::body::Incoming>,
        connection_info: Option<pavex::connection::ConnectionInfo>,
        #[allow(unused)]
        state: &ApplicationState,
    ) -> pavex::Response {
        let (request_head, request_body) = request.into_parts();
        let request_head: pavex::request::RequestHead = request_head.into();
        let request_body = pavex::request::body::RawIncomingBody::from(request_body);
        let Ok(matched_route) = self.router.at(&request_head.target.path()) else {
            let url_params = pavex::request::path::RawPathParams::default();
            let allowed_methods: pavex::router::AllowedMethods = pavex::router::MethodAllowList::from_iter(
                    vec![],
                )
                .into();
            let matched_route_template = pavex::request::path::MatchedPathPattern::new(
                "*",
            );
            let connection_info = connection_info
                .expect("Required ConnectionInfo is missing");
            return route_0::entrypoint(
                    &connection_info,
                    &request_head,
                    &request_body,
                    &allowed_methods,
                    &matched_route_template,
                    &url_params,
                )
                .await;
        };
        let url_params: pavex::request::path::RawPathParams<'_, '_> = matched_route
            .params
            .into();
        match matched_route.value {
            0u32 => {
                let allowed_methods: pavex::router::AllowedMethods = pavex::router::MethodAllowList::from_iter([])
                    .into();
                let connection_info = connection_info
                    .expect("Required `ConnectionInfo` is missing");
                let matched_route_template = pavex::request::path::MatchedPathPattern::new(
                    "/nested{*catch_all}",
                );
                route_1::entrypoint(
                        &connection_info,
                        &request_head,
                        &request_body,
                        &allowed_methods,
                        &matched_route_template,
                        &url_params,
                    )
                    .await
            }
            i => unreachable!("Unknown route id: {}", i),
        }
    }
}
pub mod route_0 {
    pub async fn entrypoint<'a, 'b, 'c, 'd, 'e, 'f, 'g, 'h>(
        s_0: &'a pavex::connection::ConnectionInfo,
        s_1: &'b pavex::request::RequestHead,
        s_2: &'c pavex::request::body::RawIncomingBody,
        s_3: &'d pavex::router::AllowedMethods,
        s_4: &'e pavex::request::path::MatchedPathPattern,
        s_5: &'h pavex::request::path::RawPathParams<'f, 'g>,
    ) -> pavex::Response {
        let response = wrapping_0(s_0, s_1, s_2, s_3, s_4, s_5).await;
        response
    }
    async fn stage_1<'a, 'b, 'c, 'd, 'e, 'f, 'g, 'h>(
        s_0: &'a pavex::connection::ConnectionInfo,
        s_1: &'b pavex::request::RequestHead,
        s_2: &'c pavex::request::body::RawIncomingBody,
        s_3: &'d pavex::router::AllowedMethods,
        s_4: &'e pavex::request::path::MatchedPathPattern,
        s_5: &'h pavex::request::path::RawPathParams<'f, 'g>,
    ) -> pavex::Response {
        let response = handler(s_0, s_1, s_2, s_3, s_4, s_5).await;
        response
    }
    async fn wrapping_0(
        v0: &pavex::connection::ConnectionInfo,
        v1: &pavex::request::RequestHead,
        v2: &pavex::request::body::RawIncomingBody,
        v3: &pavex::router::AllowedMethods,
        v4: &pavex::request::path::MatchedPathPattern,
        v5: &pavex::request::path::RawPathParams<'_, '_>,
    ) -> pavex::Response {
        let v6 = crate::route_0::Next0 {
            s_0: v0,
            s_1: v1,
            s_2: v2,
            s_3: v3,
            s_4: v4,
            s_5: v5,
            next: stage_1,
        };
        let v7 = pavex::middleware::Next::new(v6);
        let v8 = pavex::middleware::wrap_noop(v7).await;
        <pavex::Response as pavex::IntoResponse>::into_response(v8)
    }
    async fn handler(
        v0: &pavex::connection::ConnectionInfo,
        v1: &pavex::request::RequestHead,
        v2: &pavex::request::body::RawIncomingBody,
        v3: &pavex::router::AllowedMethods,
        v4: &pavex::request::path::MatchedPathPattern,
        v5: &pavex::request::path::RawPathParams<'_, '_>,
    ) -> pavex::Response {
        let v6 = app::handler(v0, v1, v2, v3, v4, v5);
        <pavex::Response as pavex::IntoResponse>::into_response(v6)
    }
    struct Next0<'a, 'b, 'c, 'd, 'e, 'f, 'g, 'h, T>
    where
        T: std::future::Future<Output = pavex::Response>,
    {
        s_0: &'a pavex::connection::ConnectionInfo,
        s_1: &'b pavex::request::RequestHead,
        s_2: &'c pavex::request::body::RawIncomingBody,
        s_3: &'d pavex::router::AllowedMethods,
        s_4: &'e pavex::request::path::MatchedPathPattern,
        s_5: &'h pavex::request::path::RawPathParams<'f, 'g>,
        next: fn(
            &'a pavex::connection::ConnectionInfo,
            &'b pavex::request::RequestHead,
            &'c pavex::request::body::RawIncomingBody,
            &'d pavex::router::AllowedMethods,
            &'e pavex::request::path::MatchedPathPattern,
            &'h pavex::request::path::RawPathParams<'f, 'g>,
        ) -> T,
    }
    impl<'a, 'b, 'c, 'd, 'e, 'f, 'g, 'h, T> std::future::IntoFuture
    for Next0<'a, 'b, 'c, 'd, 'e, 'f, 'g, 'h, T>
    where
        T: std::future::Future<Output = pavex::Response>,
    {
        type Output = pavex::Response;
        type IntoFuture = T;
        fn into_future(self) -> Self::IntoFuture {
            (self.next)(self.s_0, self.s_1, self.s_2, self.s_3, self.s_4, self.s_5)
        }
    }
}
pub mod route_1 {
    pub async fn entrypoint<'a, 'b, 'c, 'd, 'e, 'f, 'g, 'h>(
        s_0: &'a pavex::connection::ConnectionInfo,
        s_1: &'b pavex::request::RequestHead,
        s_2: &'c pavex::request::body::RawIncomingBody,
        s_3: &'d pavex::router::AllowedMethods,
        s_4: &'e pavex::request::path::MatchedPathPattern,
        s_5: &'h pavex::request::path::RawPathParams<'f, 'g>,
    ) -> pavex::Response {
        let response = wrapping_0(s_0, s_1, s_2, s_3, s_4, s_5).await;
        response
    }
    async fn stage_1<'a, 'b, 'c, 'd, 'e, 'f, 'g, 'h>(
        s_0: &'a pavex::connection::ConnectionInfo,
        s_1: &'b pavex::request::RequestHead,
        s_2: &'c pavex::request::body::RawIncomingBody,
        s_3: &'d pavex::router::AllowedMethods,
        s_4: &'e pavex::request::path::MatchedPathPattern,
        s_5: &'h pavex::request::path::RawPathParams<'f, 'g>,
    ) -> pavex::Response {
        let response = handler(s_0, s_1, s_2, s_3, s_4, s_5).await;
        response
    }
    async fn wrapping_0(
        v0: &pavex::connection::ConnectionInfo,
        v1: &pavex::request::RequestHead,
        v2: &pavex::request::body::RawIncomingBody,
        v3: &pavex::router::AllowedMethods,
        v4: &pavex::request::path::MatchedPathPattern,
        v5: &pavex::request::path::RawPathParams<'_, '_>,
    ) -> pavex::Response {
        let v6 = crate::route_1::Next0 {
            s_0: v0,
            s_1: v1,
            s_2: v2,
            s_3: v3,
            s_4: v4,
            s_5: v5,
            next: stage_1,
        };
        let v7 = pavex::middleware::Next::new(v6);
        let v8 = pavex::middleware::wrap_noop(v7).await;
        <pavex::Response as pavex::IntoResponse>::into_response(v8)
    }
    async fn handler(
        v0: &pavex::connection::ConnectionInfo,
        v1: &pavex::request::RequestHead,
        v2: &pavex::request::body::RawIncomingBody,
        v3: &pavex::router::AllowedMethods,
        v4: &pavex::request::path::MatchedPathPattern,
        v5: &pavex::request::path::RawPathParams<'_, '_>,
    ) -> pavex::Response {
        let v6 = app::handler(v0, v1, v2, v3, v4, v5);
        <pavex::Response as pavex::IntoResponse>::into_response(v6)
    }
    struct Next0<'a, 'b, 'c, 'd, 'e, 'f, 'g, 'h, T>
    where
        T: std::future::Future<Output = pavex::Response>,
    {
        s_0: &'a pavex::connection::ConnectionInfo,
        s_1: &'b pavex::request::RequestHead,
        s_2: &'c pavex::request::body::RawIncomingBody,
        s_3: &'d pavex::router::AllowedMethods,
        s_4: &'e pavex::request::path::MatchedPathPattern,
        s_5: &'h pavex::request::path::RawPathParams<'f, 'g>,
        next: fn(
            &'a pavex::connection::ConnectionInfo,
            &'b pavex::request::RequestHead,
            &'c pavex::request::body::RawIncomingBody,
            &'d pavex::router::AllowedMethods,
            &'e pavex::request::path::MatchedPathPattern,
            &'h pavex::request::path::RawPathParams<'f, 'g>,
        ) -> T,
    }
    impl<'a, 'b, 'c, 'd, 'e, 'f, 'g, 'h, T> std::future::IntoFuture
    for Next0<'a, 'b, 'c, 'd, 'e, 'f, 'g, 'h, T>
    where
        T: std::future::Future<Output = pavex::Response>,
    {
        type Output = pavex::Response;
        type IntoFuture = T;
        fn into_future(self) -> Self::IntoFuture {
            (self.next)(self.s_0, self.s_1, self.s_2, self.s_3, self.s_4, self.s_5)
        }
    }
}
