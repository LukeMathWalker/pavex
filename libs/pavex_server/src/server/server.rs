use std::net::SocketAddr;
use std::thread;

use tokio::net::TcpStream;
use tokio::sync::mpsc::error::TrySendError;

use crate::incoming::Incoming;
use crate::server::configuration::ServerConfiguration;
use crate::server::worker::{Worker, WorkerHandle};
use crate::ServerBuilder;

pub struct Server {
    config: ServerConfiguration,
    incoming: Vec<Incoming>,
}

impl Server {
    pub(super) fn new(config: ServerConfiguration, incoming: Vec<Incoming>) -> Self {
        Self { config, incoming }
    }

    /// Configure a [`Server`] using a [`ServerBuilder`].
    pub fn builder() -> ServerBuilder {
        ServerBuilder::new()
    }

    pub async fn run(self) {
        let Server { config, incoming } = self;
        let acceptor = Acceptor::new(config, incoming);
        acceptor.spawn();
    }
}

#[must_use]
struct Acceptor {
    incoming: Vec<Incoming>,
    worker_handles: Vec<WorkerHandle>,
    config: ServerConfiguration,
    next_worker: usize,
    max_queue_length: usize,
}

impl Acceptor {
    fn new(config: ServerConfiguration, incoming: Vec<Incoming>) -> Self {
        // TODO: make this configurable
        let max_queue_length = 15;
        let n_workers = config.n_workers.get();
        let mut worker_handles = Vec::with_capacity(n_workers);
        for i in 0..n_workers {
            let (worker, handle) = Worker::new(i, max_queue_length);
            worker_handles.push(handle);
            // TODO: should we panic here?
            worker.spawn().expect("Failed to spawn worker thread");
        }
        Self {
            incoming,
            worker_handles,
            config,
            max_queue_length,
            next_worker: 0,
        }
    }

    /// Run the acceptor: accept incoming connections and dispatch them to workers.
    ///
    /// Constraint: this method **must not panic**.
    async fn run(mut self) {
        /// Accept a connection from the given [`Incoming`].
        /// If accepting a certain connection fails, log the error and keep trying with the next connection.
        async fn accept_connection(incoming: Incoming) -> (Incoming, TcpStream, SocketAddr) {
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

        let n_workers = self.worker_handles.len();

        let mut incoming_join_set = tokio::task::JoinSet::new();
        for incoming in self.incoming.into_iter() {
            incoming_join_set.spawn(accept_connection(incoming));
        }

        let error = 'conn_loop: loop {
            let (incoming, mut connection, remote_peer) = match incoming_join_set.join_next().await
            {
                Some(Ok((incoming, connection, remote_peer))) => {
                    (incoming, connection, remote_peer)
                }
                Some(Err(e)) => {
                    // This only ever happens if we panicked in the task that was accepting
                    // connections or if we somehow cancel it.
                    // Neither of these should ever happen, but we handle the error just in case
                    // to make sure we log the error info if we end up introducing a fatal bug.
                    break 'conn_loop e;
                }
                None => {
                    // When we succeed in accepting a connection, we always spawn a new task to
                    // accept the next connection from the same socket.
                    // If we fail to accept a connection, we exit the acceptor thread.
                    // Therefore, the JoinSet should never be empty.
                    unreachable!("The JoinSet for incoming connections cannot ever be empty")
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
                let worker_handle = &self.worker_handles[self.next_worker];
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
                    self.next_worker = (self.next_worker + 1) % n_workers;
                } else {
                    // We've successfully sent the connection to a worker, so we can stop trying
                    // to send it to other workers.
                    has_been_handled = true;
                    break;
                }

                // Restart the crashed worker thread.
                if let Some(worker_id) = has_crashed {
                    tracing::warn!(worker_id = worker_id, "Worker crashed, restarting it");
                    let (worker, worker_handle) = Worker::new(worker_id, self.max_queue_length);
                    // TODO: what if we fail to spawn the worker thread? We don't want to panic here!
                    worker.spawn().expect("Failed to spawn worker thread");
                    self.worker_handles[worker_id] = worker_handle;
                }
            }

            if !has_been_handled {
                tracing::error!(
                    remote_peer = %remote_peer,
                    "All workers are busy, dropping connection",
                );
            }
        };

        tracing::error!(
            error.msg = %error,
            error.details = ?error,
            "Failed to accept new connections. The acceptor thread will exit now."
        );
    }

    fn spawn(self) -> thread::JoinHandle<()> {
        thread::Builder::new()
            .name("pavex-acceptor".to_string())
            .spawn(move || {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to build single-threaded Tokio runtime for acceptor thread")
                    .block_on(self.run());
            })
            .expect("Failed to spawn acceptor thread")
    }
}
