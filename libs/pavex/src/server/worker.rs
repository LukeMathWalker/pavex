use std::future::{Future, poll_fn};
use std::net::SocketAddr;
use std::task::Poll;
use std::thread;

use anyhow::Context;
use hyper_util::rt::TokioIo;
use hyper_util::server::graceful::GracefulShutdown;
use tokio::net::TcpStream;
use tokio::sync::mpsc::error::TrySendError;
use tracing_log_error::log_error;

use crate::connection::ConnectionInfo;
use crate::server::ShutdownMode;

pub(super) struct ConnectionMessage {
    pub(super) connection: TcpStream,
    pub(super) peer_addr: SocketAddr,
}

/// A handle to dispatch incoming connections to a worker thread.
pub(super) struct WorkerHandle {
    connection_outbox: tokio::sync::mpsc::Sender<ConnectionMessage>,
    // We use an unbounded channel because we want to be able to send a shutdown command
    // synchronously.
    shutdown_outbox: tokio::sync::mpsc::UnboundedSender<ShutdownWorkerCommand>,
    id: usize,
}

impl WorkerHandle {
    /// Dispatch a connection to the worker thread.
    pub(super) fn dispatch(
        &self,
        connection: ConnectionMessage,
    ) -> Result<(), TrySendError<ConnectionMessage>> {
        self.connection_outbox.try_send(connection)
    }

    /// Get the worker's ID.
    pub(super) fn id(&self) -> usize {
        self.id
    }

    /// Shutdown the worker thread.
    ///
    /// # Implementation notes
    ///
    /// We use a sync function to ensure that the shutdown command is enqueued immediately,
    /// even if the returned future is never polled.
    pub(super) fn shutdown(self, mode: ShutdownMode) -> impl Future<Output = ()> {
        let (completion_notifier, completion) = tokio::sync::oneshot::channel();
        let sent = self
            .shutdown_outbox
            .send(ShutdownWorkerCommand {
                completion_notifier,
                mode,
            })
            .is_ok();
        async move {
            // What if sending fails?
            // It only happens if the other end of the channel has already been dropped, which
            // implies that the worker thread has already shut down—nothing to do!
            if sent {
                let _ = completion.await;
            }
        }
    }
}

pub(super) struct ShutdownWorkerCommand {
    completion_notifier: tokio::sync::oneshot::Sender<()>,
    mode: ShutdownMode,
}

#[must_use]
/// A worker thread that handles incoming connections.
pub(super) struct Worker<HandlerFuture, ApplicationState> {
    connection_inbox: tokio::sync::mpsc::Receiver<ConnectionMessage>,
    shutdown_inbox: tokio::sync::mpsc::UnboundedReceiver<ShutdownWorkerCommand>,
    handler: fn(
        http::Request<hyper::body::Incoming>,
        Option<ConnectionInfo>,
        ApplicationState,
    ) -> HandlerFuture,
    application_state: ApplicationState,
    id: usize,
    shutdown_coordinator: GracefulShutdown,
}

impl<HandlerFuture, ApplicationState> Worker<HandlerFuture, ApplicationState>
where
    HandlerFuture: Future<Output = crate::response::Response> + 'static,
    ApplicationState: Clone + Send + Sync + 'static,
{
    /// Configure a new worker without spawning it.
    ///
    /// `max_queue_length` is the maximum number of connections that can be queued up for this
    /// worker.
    pub(super) fn new(
        id: usize,
        max_queue_length: usize,
        handler: fn(
            http::Request<hyper::body::Incoming>,
            Option<ConnectionInfo>,
            ApplicationState,
        ) -> HandlerFuture,
        application_state: ApplicationState,
    ) -> (Self, WorkerHandle) {
        let (connection_outbox, connection_inbox) = tokio::sync::mpsc::channel(max_queue_length);
        let (shutdown_outbox, shutdown_inbox) = tokio::sync::mpsc::unbounded_channel();
        let self_ = Self {
            connection_inbox,
            shutdown_inbox,
            handler,
            application_state,
            id,
            shutdown_coordinator: GracefulShutdown::new(),
        };
        let handle = WorkerHandle {
            connection_outbox,
            shutdown_outbox,
            id,
        };
        (self_, handle)
    }

    /// Spawn a thread and run the worker there, using a single-threaded executor that can
    /// handle !Send futures.
    pub(super) fn spawn(self) -> Result<thread::JoinHandle<()>, anyhow::Error> {
        let id = self.id;
        let name = || format!("pavex-worker-{id}");
        thread::Builder::new()
            .name(name())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    // This can only fail if `tokio` fails to create an IO stack for the
                    // current thread executor, which is highly unlikely to happen
                    // and probably indicates a deeper issue that we can't recover from.
                    .expect("Failed to build single-threaded Tokio runtime for worker thread");

                // Use a `LocalSet` to allow the worker to spawn !Send futures.
                let local = tokio::task::LocalSet::new();
                local.block_on(&runtime, self.run());
            })
            .with_context(|| format!("Failed to spawn worker thread `{}`", name()))
    }

    /// Run the worker: wait for incoming connections and handle them.
    async fn run(self) {
        let Self {
            mut connection_inbox,
            mut shutdown_inbox,
            handler,
            application_state,
            id,
            shutdown_coordinator,
        } = self;
        'event_loop: loop {
            let message =
                poll_fn(|cx| Self::poll_inboxes(cx, &mut shutdown_inbox, &mut connection_inbox))
                    .await;
            match message {
                WorkerInboxMessage::Connection(connection) => {
                    Self::handle_connection(
                        connection,
                        handler,
                        application_state.clone(),
                        &shutdown_coordinator,
                    );
                }
                WorkerInboxMessage::Shutdown(shutdown) => {
                    let ShutdownWorkerCommand {
                        completion_notifier,
                        mode,
                    } = shutdown;
                    match mode {
                        ShutdownMode::Graceful { timeout } => {
                            // Stop accepting new connections.
                            connection_inbox.close();

                            // Kick-off work for all pending connections.
                            while let Some(connection) = connection_inbox.recv().await {
                                Self::handle_connection(
                                    connection,
                                    handler,
                                    application_state.clone(),
                                    &shutdown_coordinator,
                                );
                            }

                            // Wait for all live connections to be closed or for the timeout to expire.
                            let _ = tokio::time::timeout(timeout, shutdown_coordinator.shutdown())
                                .await;
                        }
                        ShutdownMode::Forced => {}
                    }
                    let _ = completion_notifier.send(());
                    break 'event_loop;
                }
            }
        }
        tracing::info!(worker_id = id, "Worker shut down");
    }

    fn handle_connection(
        connection_message: ConnectionMessage,
        handler: fn(
            http::Request<hyper::body::Incoming>,
            Option<ConnectionInfo>,
            ApplicationState,
        ) -> HandlerFuture,
        application_state: ApplicationState,
        shutdown_coordinator: &GracefulShutdown,
    ) {
        let ConnectionMessage {
            connection,
            peer_addr,
        } = connection_message;
        // A tiny bit of glue to adapt our handler to hyper's service interface.
        let handler = hyper::service::service_fn(move |request| {
            let state = application_state.clone();

            async move {
                let handler = (handler)(request, Some(ConnectionInfo { peer_addr }), state);
                let response = handler.await;
                let response = hyper::Response::from(response);
                Ok::<_, hyper::Error>(response)
            }
        });
        // TODO: expose all the config options for `auto::Builder` through the top-level
        //   `ServerConfiguration` object.
        let builder = hyper_util::server::conn::auto::Builder::new(LocalExec);
        let connection = TokioIo::new(connection);
        let connection_future =
            shutdown_coordinator.watch(builder.serve_connection(connection, handler).into_owned());
        tokio::task::spawn_local(async move {
            if let Err(e) = connection_future.await {
                log_error!(*e, level: tracing::Level::WARN, "Failed to serve an incoming connection");
            }
        });
    }

    /// Check if there is work to be done.
    fn poll_inboxes(
        cx: &mut std::task::Context<'_>,
        shutdown_inbox: &mut tokio::sync::mpsc::UnboundedReceiver<ShutdownWorkerCommand>,
        connection_inbox: &mut tokio::sync::mpsc::Receiver<ConnectionMessage>,
    ) -> Poll<WorkerInboxMessage> {
        // Order matters here: we want to prioritize shutdown messages over incoming connections.
        if let Poll::Ready(Some(message)) = shutdown_inbox.poll_recv(cx) {
            return Poll::Ready(message.into());
        }
        if let Poll::Ready(Some(message)) = connection_inbox.poll_recv(cx) {
            return Poll::Ready(message.into());
        }
        Poll::Pending
    }
}

enum WorkerInboxMessage {
    Connection(ConnectionMessage),
    Shutdown(ShutdownWorkerCommand),
}

impl From<ConnectionMessage> for WorkerInboxMessage {
    fn from(connection: ConnectionMessage) -> Self {
        Self::Connection(connection)
    }
}

impl From<ShutdownWorkerCommand> for WorkerInboxMessage {
    fn from(command: ShutdownWorkerCommand) -> Self {
        Self::Shutdown(command)
    }
}

/// HTTP2 requires `hyper` to be able to spawn tasks, therefore we need to pass to `hyper`'s
/// `Server` an executor and a way to perform the spawning.
///
/// We use `spawn_local` since we want each worker thread to be able to spawn !Send futures.
#[derive(Clone, Copy, Debug)]
struct LocalExec;

impl<F> hyper::rt::Executor<F> for LocalExec
where
    F: Future + 'static, // no `Send`
{
    fn execute(&self, fut: F) {
        // This will spawn into the currently running `LocalSet`.
        tokio::task::spawn_local(fut);
    }
}
