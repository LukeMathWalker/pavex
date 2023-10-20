use std::future::{poll_fn, Future, IntoFuture};
use std::marker::PhantomData;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::Poll;
use std::thread;

use tokio::net::TcpStream;
use tokio::sync::mpsc::error::TrySendError;
use tokio::task::{JoinError, JoinSet, LocalSet};

use crate::server::configuration::ServerConfiguration;
use crate::server::worker::{Worker, WorkerHandle};

use super::{IncomingStream, ShutdownMode};

/// A handle to a running [`Server`](super::Server).
///
/// # Example: waiting for the server to shut down
///
/// You can just `.await` the [`ServerHandle`] to wait for the server to shut down:
///
/// ```rust
/// use std::net::SocketAddr;
/// use pavex::server::Server;
///
/// # #[derive(Clone)] struct ApplicationState;
/// # async fn router(_req: hyper::Request<hyper::body::Incoming>, _state: ApplicationState) -> pavex::response::Response { todo!() }
/// # async fn t() -> std::io::Result<()> {
/// # let application_state = ApplicationState;
/// let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
///
/// let server_handle = Server::new()
///     .bind(addr)
///     .await?
///     .serve(router, application_state);
/// // Wait until the server shuts down.
/// server_handle.await;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct ServerHandle {
    command_outbox: tokio::sync::mpsc::Sender<ServerCommand>,
}

impl ServerHandle {
    pub(super) fn new<HandlerFuture, ApplicationState>(
        config: ServerConfiguration,
        incoming: Vec<IncomingStream>,
        handler: fn(http::Request<hyper::body::Incoming>, ApplicationState) -> HandlerFuture,
        application_state: ApplicationState,
    ) -> Self
    where
        HandlerFuture: Future<Output = crate::response::Response> + 'static,
        ApplicationState: Clone + Send + Sync + 'static,
    {
        let (command_outbox, command_inbox) = tokio::sync::mpsc::channel(32);
        let acceptor = Acceptor::new(config, incoming, handler, application_state, command_inbox);
        let _ = acceptor.spawn();
        Self { command_outbox }
    }

    /// Instruct the [`Server`](super::Server) to stop accepting new connections.
    #[doc(alias("stop"))]
    pub async fn shutdown(self, mode: ShutdownMode) {
        let (completion_notifier, completion) = tokio::sync::oneshot::channel();
        if self
            .command_outbox
            .send(ServerCommand::Shutdown {
                completion_notifier,
                mode,
            })
            .await
            .is_ok()
        {
            // What if sending fails?
            // It only happens if the other end of the channel has already been dropped, which
            // implies that the acceptor thread has already shut down—nothing to do!
            let _ = completion.await;
        }
    }
}

impl IntoFuture for ServerHandle {
    type Output = ();
    type IntoFuture = Pin<Box<dyn Future<Output = ()> + Send + Sync + 'static>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move { self.command_outbox.closed().await })
    }
}

enum ServerCommand {
    Shutdown {
        completion_notifier: tokio::sync::oneshot::Sender<()>,
        mode: ShutdownMode,
    },
}

#[must_use]
struct Acceptor<HandlerFuture, ApplicationState> {
    command_inbox: tokio::sync::mpsc::Receiver<ServerCommand>,
    incoming: Vec<IncomingStream>,
    worker_handles: Vec<WorkerHandle>,
    #[allow(dead_code)]
    config: ServerConfiguration,
    next_worker: usize,
    max_queue_length: usize,
    handler: fn(http::Request<hyper::body::Incoming>, ApplicationState) -> HandlerFuture,
    application_state: ApplicationState,
    // We use a `fn() -> HandlerFuture` instead of a `HandlerFuture` because we need `Acceptor`
    // to be `Send` and `Sync`. That wouldn't work with `PhantomData<HandlerFuture>`.
    // In the end, we just need to stash the generic type *somewhere*.
    handler_output_future: PhantomData<fn() -> HandlerFuture>,
}

enum AcceptorInboxMessage {
    ServerCommand(ServerCommand),
    Connection(Option<Result<(IncomingStream, TcpStream, SocketAddr), JoinError>>),
}

impl<HandlerFuture, ApplicationState> Acceptor<HandlerFuture, ApplicationState>
where
    HandlerFuture: Future<Output = crate::response::Response> + 'static,
    ApplicationState: Clone + Send + Sync + 'static,
{
    fn new(
        config: ServerConfiguration,
        incoming: Vec<IncomingStream>,
        handler: fn(http::Request<hyper::body::Incoming>, ApplicationState) -> HandlerFuture,
        application_state: ApplicationState,
        command_inbox: tokio::sync::mpsc::Receiver<ServerCommand>,
    ) -> Self {
        // TODO: make this configurable
        let max_queue_length = 15;
        let n_workers = config.n_workers.get();
        let mut worker_handles = Vec::with_capacity(n_workers);
        for i in 0..n_workers {
            let (worker, handle) =
                Worker::new(i, max_queue_length, handler, application_state.clone());
            worker_handles.push(handle);
            // TODO: should we panic here?
            worker.spawn().expect("Failed to spawn worker thread");
        }
        Self {
            command_inbox,
            incoming,
            worker_handles,
            config,
            max_queue_length,
            handler,
            handler_output_future: Default::default(),
            next_worker: 0,
            application_state,
        }
    }

    /// Run the acceptor: accept incoming connections and dispatch them to workers.
    ///
    /// Constraint: this method **must not panic**.
    async fn run(self) {
        /// Accept a connection from the given [`IncomingStream`].
        /// If accepting a certain connection fails, log the error and keep trying with the next connection.
        async fn accept_connection(
            incoming: IncomingStream,
        ) -> (IncomingStream, TcpStream, SocketAddr) {
            loop {
                match incoming.accept().await {
                    Ok((connection, remote_peer)) => return (incoming, connection, remote_peer),
                    Err(e) => {
                        tracing::error!(error.msg = %e, error.details = ?e, "Failed to accept connection");
                        continue;
                    }
                }
            }
        }

        let Self {
            mut command_inbox,
            mut next_worker,
            mut worker_handles,
            incoming,
            config: _,
            max_queue_length,
            handler,
            application_state,
            handler_output_future: _,
        } = self;

        let n_workers = worker_handles.len();

        let mut incoming_join_set = JoinSet::new();
        for incoming in incoming.into_iter() {
            incoming_join_set.spawn(accept_connection(incoming));
        }

        let error = 'event_loop: loop {
            // Check if there is work to be done.
            let message =
                poll_fn(|cx| Self::poll_inboxes(cx, &mut command_inbox, &mut incoming_join_set))
                    .await;
            match message {
                AcceptorInboxMessage::ServerCommand(command) => match command {
                    ServerCommand::Shutdown {
                        completion_notifier,
                        mode,
                    } => {
                        Self::shutdown(
                            completion_notifier,
                            mode,
                            incoming_join_set,
                            worker_handles,
                        )
                        .await;
                        return;
                    }
                },
                AcceptorInboxMessage::Connection(msg) => {
                    let (incoming, mut connection, remote_peer) = match msg {
                        Some(Ok((incoming, connection, remote_peer))) => {
                            (incoming, connection, remote_peer)
                        }
                        Some(Err(e)) => {
                            // This only ever happens if we panicked in the task that was accepting
                            // connections or if we somehow cancel it.
                            // Neither of these should ever happen, but we handle the error just in case
                            // to make sure we log the error info if we end up introducing a fatal bug.
                            break 'event_loop e;
                        }
                        None => {
                            // When we succeed in accepting a connection, we always spawn a new task to
                            // accept the next connection from the same socket.
                            // If we fail to accept a connection, we exit the acceptor thread.
                            // Therefore, the JoinSet should never be empty.
                            unreachable!(
                                "The JoinSet for incoming connections cannot ever be empty"
                            )
                        }
                    };
                    // Re-spawn the task to keep accepting connections from the same socket.
                    incoming_join_set.spawn(accept_connection(incoming));

                    // A flag to track if the connection has been successfully sent to a worker.
                    let mut has_been_handled = false;
                    // We try to send the connection to a worker.
                    // If the worker's inbox is full, we try the next worker until we find one that can
                    // accept the connection or we've tried all workers.
                    for _ in 0..n_workers {
                        // Track if the worker has crashed.
                        let mut has_crashed: Option<usize> = None;
                        let worker_handle = &worker_handles[next_worker];
                        if let Err(e) = worker_handle.dispatch(connection) {
                            connection = match e {
                                TrySendError::Full(conn) => conn,
                                // A closed channel implies that the worker thread is no longer running,
                                // therefore we need to restart it.
                                TrySendError::Closed(conn) => {
                                    has_crashed = Some(worker_handle.id());
                                    conn
                                }
                            };
                            next_worker = (next_worker + 1) % n_workers;
                        } else {
                            // We've successfully sent the connection to a worker, so we can stop trying
                            // to send it to other workers.
                            has_been_handled = true;
                            break;
                        }

                        // Restart the crashed worker thread.
                        if let Some(worker_id) = has_crashed {
                            tracing::warn!(worker_id = worker_id, "Worker crashed, restarting it");
                            let (worker, worker_handle) = Worker::new(
                                worker_id,
                                max_queue_length,
                                handler,
                                application_state.clone(),
                            );
                            // TODO: what if we fail to spawn the worker thread? We don't want to panic here!
                            worker.spawn().expect("Failed to spawn worker thread");
                            worker_handles[worker_id] = worker_handle;
                        }
                    }

                    if !has_been_handled {
                        tracing::error!(
                            remote_peer = %remote_peer,
                            "All workers are busy, dropping connection",
                        );
                    }
                }
            }
        };

        tracing::error!(
            error.msg = %error,
            error.details = ?error,
            "Failed to accept new connections. The acceptor thread will exit now."
        );
    }

    /// Check if there is work to be done.
    fn poll_inboxes(
        cx: &mut std::task::Context<'_>,
        server_command_inbox: &mut tokio::sync::mpsc::Receiver<ServerCommand>,
        incoming_join_set: &mut JoinSet<(IncomingStream, TcpStream, SocketAddr)>,
    ) -> Poll<AcceptorInboxMessage> {
        // Order matters here: we want to prioritize shutdown messages over incoming connections.
        if let Poll::Ready(Some(message)) = server_command_inbox.poll_recv(cx) {
            return Poll::Ready(AcceptorInboxMessage::ServerCommand(message));
        }
        if let Poll::Ready(message) = incoming_join_set.poll_join_next(cx) {
            return Poll::Ready(AcceptorInboxMessage::Connection(message));
        }
        Poll::Pending
    }

    fn spawn(self) -> thread::JoinHandle<()> {
        thread::Builder::new()
            .name("pavex-acceptor".to_string())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to build single-threaded Tokio runtime for acceptor thread");
                LocalSet::new().block_on(&rt, self.run());
            })
            .expect("Failed to spawn acceptor thread")
    }

    async fn shutdown(
        completion_notifier: tokio::sync::oneshot::Sender<()>,
        mode: ShutdownMode,
        incoming_join_set: JoinSet<(IncomingStream, TcpStream, SocketAddr)>,
        worker_handles: Vec<WorkerHandle>,
    ) {
        // This drops the `JoinSet`, which will cause all the tasks that are still running to
        // be cancelled.
        // It will in turn cause the `Incoming` to be dropped, which will cause the `TcpListener`
        // to be dropped, thus closing the socket and stopping acceptance of new connections.
        drop(incoming_join_set);

        let mut shutdown_join_set = JoinSet::new();
        for worker_handle in worker_handles {
            let mode2 = mode.clone();
            // The shutdown command is enqueued immediately, before the future is polled for the
            // first time.
            let future = worker_handle.shutdown(mode2);
            if mode.is_graceful() {
                shutdown_join_set.spawn_local(future);
            }
        }

        if let ShutdownMode::Graceful { timeout } = mode {
            // Wait for all workers to shut down, or for the timeout to expire,
            // whichever happens first.
            let _ = tokio::time::timeout(timeout, async move {
                while let Some(_) = shutdown_join_set.join_next().await {}
            })
            .await;
        }

        // Notify the caller that the server has shut down.
        let _ = completion_notifier.send(());
    }
}
