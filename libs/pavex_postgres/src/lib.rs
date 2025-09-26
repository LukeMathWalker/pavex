use camino::Utf8PathBuf as PathBuf;
use pavex::tls::client::TlsClientPolicyConfig;
use redact::Secret;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// All the parameters required to establish a connection to a PostgreSQL database.
///
/// [`PgConfig`] is driver-agnostic. You can use it to build a connection
/// via your favourite Rust PostgreSQL library, such as [`sqlx`](https://docs.rs/sqlx/latest/sqlx/),
/// [`diesel`](https://docs.rs/diesel/latest/diesel/) or [`tokio-postgres`](https://docs.rs/tokio-postgres/latest/tokio_postgres/).
///
/// # `libpq`
///
/// Most parameters in [`PgConfig`] have a direct counterpart in
/// [`libpq`](https://www.postgresql.org/docs/current/libpq.html).
/// The documentation includes a link to the documentation of the corresponding `libpq` parameter when available.
///
/// We primarily diverge from `libpq` when it comes to TLS configuration, where the naming in
/// [`PgConfig`] is more modern and defaults are stricter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PgConfig {
    /// Where to connect.
    ///
    /// You can connect to a TCP endpoint (or more than one!), as well as to a UNIX domain socket.
    /// Check out [`Endpoint`]'s documentation for more details.
    ///
    /// # `libpq`
    ///
    /// `endpoint` groups together the following configuration parameters from `libpq`:
    /// - [`host`](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNECT-HOST)
    /// - [`hostaddr`](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNECT-HOSTADDR)
    /// - [`port`](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNSTRING)
    /// - [`load_balance_hosts`](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNECT-LOAD-BALANCE-HOSTS) (PG ≥ 14).
    pub endpoint: EndpointConfig,

    /// Which database to connect to.
    ///
    /// # `libpq`
    ///
    /// `database_name` maps to `libpq`'s [`dbname`](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNECT-DBNAME).
    pub database_name: String,

    /// User to authenticate as.
    ///
    /// # `libpq`
    ///
    /// `username` maps to `libpq`'s [`user`](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNECT-USER).
    pub username: String,

    /// Password for authentication.
    ///
    /// # `libpq`
    ///
    /// `password` maps to `libpq`'s [`password`](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNECT-PASSWORD).
    #[serde(serialize_with = "redact::expose_secret", default)]
    pub password: Option<Secret<String>>,

    /// Client identity visible in `pg_stat_activity`.
    ///
    /// # `libpq`
    ///
    /// `application_name` maps to `libpq`'s [`application_name`](https://www.postgresql.org/docs/current/runtime-config-logging.html#GUC-APPLICATION-NAME).
    pub application_name: Option<String>,

    /// Timeout for establishing the connection.
    ///
    /// # `libpq`
    ///
    /// `connect_timeout` maps to `libpq`'s [`connect_timeout`](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNECT-CONNECT-TIMEOUT) (in seconds).
    pub connect_timeout: Option<Duration>,

    /// Preferred server role/attributes when multiple hosts are provided.
    ///
    /// # `libpq`
    ///
    /// `target_session_attrs` maps to `libpq`'s [`target_session_attrs`](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNECT-TARGET-SESSION-ATTRS).
    #[serde(default)]
    pub target_session_attrs: TargetSessionAttrs,
}

// ======================================================================================
// Endpoint and per-host types
// ======================================================================================

/// Where to connect to.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EndpointConfig {
    /// TCP endpoint(s).
    Tcp(TcpEndpointConfig),

    /// UNIX-domain socket.
    UdsSocket(UdsEndpointConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdsEndpointConfig {
    /// Directory that contains the `.s.PGSQL.<port>` socket file(s).
    ///
    /// # `libpq` naming
    ///
    /// `host=/path` (URI query).
    /// <https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNSTRING>
    dir: PathBuf,

    /// Port number used for the socket filename.
    ///
    /// # `libpq` naming
    ///
    /// `port` (URI query).
    /// <https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNSTRING>
    port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpEndpointConfig {
    /// Primary hostname (for DNS/TLS name check) and display.
    ///
    /// # `libpq`
    ///
    /// `host` maps to `libpq`'s [`host`](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNECT-HOST).
    host: String,

    /// Primary port.
    ///
    /// # `libpq`
    ///
    /// `port` maps to `libpq`'s [`port`](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNECT-PORT).
    port: u16,

    /// Primary numeric address to dial, skipping DNS.
    ///
    /// # `libpq`
    ///
    /// `hostaddr` maps to `libpq`'s [`hostaddr`](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNECT-HOSTADDR).
    hostaddr: Option<String>,

    /// Additional ordered TCP endpoints (try-next or randomized if load-balanced).
    ///
    /// # `libpq`
    ///
    /// Subsequent elements of [`host`, `hostaddr`, and `port` lists](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-MULTIPLE-HOSTS).
    #[serde(default)]
    alternates: Vec<AlternateTcpEndpoint>,

    /// Randomize among hosts (PG ≥ 14).
    ///
    /// # `libpq` naming
    ///
    /// `load_balance_hosts` maps to `libpq`'s [`load_balance_hosts`](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNECT-LOAD-BALANCE-HOSTS).
    load_balance_hosts: bool,

    /// Whether (and how) the client should establish a secure SSL TCP/IP connection with the server.
    ///
    /// Check out [`SslConfig`]'s documentation for more details.
    ///
    /// # `libpq`
    ///
    /// `ssl` groups together the following configuration parameters from `libpq`:
    /// - [`sslmode`](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNECT-SSLMODE)
    /// - [`sslcert`](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNECT-SSLCERT)
    /// - [`sslrootcert`](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNECT-SSLROOTCERT)
    /// - [`sslkey`](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNECT-SSLKEY)
    /// - [`sslcrl`](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNECT-SSLCRL)
    /// - [`sslcrldir`](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNECT-SSLCRLDIR)
    #[serde(default)]
    tls: TlsClientPolicyConfig,

    /// Enable/disable TCP keepalives.
    ///
    /// # `libpq`
    ///
    /// `keepalives` maps to `libpq`'s [`keepalives`](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-KEEPALIVES).
    keepalives: Option<bool>,

    /// Idle time before first keepalive (seconds).
    ///
    /// # `libpq`
    ///
    /// `keepalives_idle` maps to `libpq`'s [`keepalives_idle`](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-KEEPALIVES).
    keepalives_idle: Option<Duration>,

    /// Interval between keepalives (seconds).
    ///
    /// # `libpq`
    ///
    /// `keepalives_interval` maps to `libpq`'s [`keepalives_interval`](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-KEEPALIVES).
    keepalives_interval: Option<Duration>,

    /// Number of failed probes before the connection is dropped.
    ///
    /// # `libpq`
    ///
    /// `keepalives_count` maps to `libpq`'s [`keepalives_count`](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-KEEPALIVES).
    keepalives_count: Option<u32>,
}

/// One alternate TCP endpoint in a multi-host setup.
///
/// # `libpq` naming
///
/// Elements of the `host`, `hostaddr`, and `port` lists after the first.
/// <https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-MULTIPLE-HOSTS>
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlternateTcpEndpoint {
    /// Secondary hostname (for DNS/TLS name check) and display.
    ///
    /// # `libpq` naming
    ///
    /// Element of `host` list.
    /// <https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNSTRING>
    pub host: String,

    /// Alternate port.
    ///
    /// If unspecified, it defaults to either the primary port or the default server port.
    ///
    /// # `libpq` naming
    ///
    /// Element of `port` list.
    /// <https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNSTRING>
    pub port: Option<u16>,

    /// Alternate numeric IP (to bypass DNS).
    ///
    /// # `libpq` naming
    ///
    /// Element of `hostaddr` list (numeric IP).
    /// <https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNSTRING>
    pub hostaddr: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum LoadBalanceHosts {
    #[default]
    Disable,
    Random,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum TargetSessionAttrs {
    #[default]
    Any,
    ReadWrite,
    ReadOnly,
    Primary,
    Standby,
    PreferStandby,
}
