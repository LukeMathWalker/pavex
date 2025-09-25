use camino::Utf8PathBuf as PathBuf;
use redact::Secret;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use urlencoding::Encoded;

/// All the parameters required to establish a connection to a PostgreSQL database.
///
/// [`PgConfig`] is driver-agnostic. You can use it to build a connection
/// via your favourite Rust PostgreSQL library, such as `sqlx` or `diesel`.
///
/// # Naming
///
/// [`PgConfig`] follows [`libpq`](https://www.postgresql.org/docs/current/libpq.html)'s naming conventions.
/// Divergences are extremely minor—e.g. we've expanded some parameter names to improve clarity.
///
/// The documentation for each configuration parameter includes a link to the documentation of
/// the corresponding `libpq` parameter.
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
    pub endpoint: Endpoint,

    /// Which database to use.
    ///
    /// # `libpq`
    ///
    /// `database_name` maps to `libpq`'s [`dbname`](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNECT-DBNAME).
    pub database_name: String,

    /// User to authenticate as.
    ///
    /// # `libpq`
    ///
    /// `user` maps to `libpq`'s [`user`](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNECT-USER).
    pub user: String,

    /// Password for authentication.
    ///
    /// # `libpq`
    ///
    /// `password` maps to `libpq`'s [`password`](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNECT-PASSWORD).
    #[serde(serialize_with = "redact::expose_secret", default)]
    pub password: Option<Secret<String>>,

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
    pub ssl: SslConfig,

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

    /// A mechanism to specify additional connection parameters that are not explicitly modeled by
    /// the existing fields in [`PgConfig`].
    ///
    /// Both keys and values will be percent-encoded before being appended as query parameters to the generated
    /// [connection string](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNSTRING).
    #[serde(default)]
    pub extra_params: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SslConfig {
    /// TLS behavior.
    ///
    /// # `libpq`
    ///
    /// `mode` maps to `libpq`'s [`sslmode`](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNECT-SSLMODE).
    #[serde(default)]
    pub mode: SslMode,

    /// Root CA certificate file.
    ///
    /// # `libpq`
    ///
    /// `root_cert` maps to `libpq`'s [`sslrootcert`](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNECT-SSLROOTCERT).
    pub root_cert: Option<PathBuf>,

    /// Client certificate file.
    ///
    /// # `libpq`
    ///
    /// `client_cert` maps to `libpq`'s [`sslcert`](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNECT-SSLCERT).
    pub client_cert: Option<PathBuf>,

    /// Client private key file.
    ///
    /// # `libpq`
    ///
    /// `client_key` maps to `libpq`'s [`sslkey`](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNECT-SSLKEY).
    pub client_key: Option<PathBuf>,

    /// The name of the certificate revocation list file.
    ///
    /// # `libpq`
    ///
    /// `crl` maps to `libpq`'s [`sslcrl`](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNECT-SSLCRL).
    pub crl: Option<PathBuf>,

    /// Path to a certificate revocation list directory (PG ≥ 15).
    ///
    /// # `libpq`
    ///
    /// `crl_dir` maps to `libpq`'s [`sslcrldir`](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNECT-SSLCRLDIR).
    pub crl_dir: Option<PathBuf>,
}

// ======================================================================================
// Endpoint and per-host types
// ======================================================================================

/// Connection endpoint(s).
///
/// # `libpq` naming
///
/// Covers `host`, `hostaddr`, `port` and `load_balance_hosts` (PG ≥ 14).
/// <https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-MULTIPLE-HOSTS>
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Endpoint {
    /// TCP endpoint(s).
    ///
    /// # `libpq` naming
    ///
    /// `host`, `hostaddr`, `port`, [`load_balance_hosts`](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-MULTIPLE-HOSTS).
    Tcp {
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
        alternates: Vec<TcpEndpoint>,

        /// Randomize among hosts (PG ≥ 14).
        ///
        /// # `libpq` naming
        ///
        /// `load_balance_hosts` maps to `libpq`'s [`load_balance_hosts`](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNECT-LOAD-BALANCE-HOSTS).
        load_balance_hosts: bool,

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
    },

    /// Single UNIX-domain socket directory.
    ///
    /// # `libpq` naming
    ///
    /// Encoded as `host=/path` (query parameter in URI form) and `port` query parameter.
    /// <https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNSTRING>
    Socket {
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
    },
}

/// One alternate TCP endpoint in a multi-host setup.
///
/// # `libpq` naming
///
/// Elements of the `host`, `hostaddr`, and `port` lists after the first.
/// <https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-MULTIPLE-HOSTS>
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpEndpoint {
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
#[serde(rename_all = "kebab-case")]
pub enum LoadBalanceHosts {
    #[default]
    Disable,
    Random,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum SslMode {
    Disable,
    #[default]
    Prefer,
    Require,
    VerifyCa,
    VerifyFull,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum TargetSessionAttrs {
    #[default]
    Any,
    ReadWrite,
    ReadOnly,
    Primary,
    Standby,
    PreferStandby,
}

impl PgConfig {
    /// Build a [`libpq`-compatible connection string](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNSTRING).
    pub fn connection_string(&self) -> String {
        use std::fmt::Write as _;

        let mut uri = String::with_capacity(64);
        uri.push_str("postgresql://");

        self.write_userinfo(&mut uri);
        match &self.endpoint {
            Endpoint::Socket { dir, port } => {
                // Sockets → host=/path in query parameters
                write!(
                    &mut uri,
                    "/{}?host={}&port={}",
                    Encoded(&self.database_name),
                    Encoded(dir.as_str()),
                    port
                )
                .unwrap();
            }
            Endpoint::Tcp {
                host,
                port,
                hostaddr,
                alternates,
                load_balance_hosts,
                keepalives,
                keepalives_idle,
                keepalives_interval,
                keepalives_count,
            } => {
                let mut any_addr = hostaddr.is_some();

                write!(&mut uri, "/{}?", Encoded(&self.database_name)).unwrap();

                write!(&mut uri, "host={}", Encoded(host)).unwrap();
                for alternate in alternates {
                    write!(&mut uri, ",{}", Encoded(&alternate.host),).unwrap();
                    any_addr |= alternate.hostaddr.is_some();
                }

                write!(&mut uri, "&port={port}").unwrap();
                for alternate in alternates {
                    if let Some(port) = alternate.port {
                        write!(&mut uri, ",{}", port).unwrap();
                    } else {
                        uri.push(',');
                    }
                }

                if any_addr {
                    write!(
                        &mut uri,
                        "&hostaddr={}",
                        Encoded(hostaddr.as_deref().unwrap_or_default())
                    )
                    .unwrap();
                    for alternate in alternates {
                        if let Some(hostaddr) = &alternate.hostaddr {
                            write!(&mut uri, ",{}", Encoded(hostaddr)).unwrap();
                        } else {
                            uri.push(',');
                        }
                    }
                }

                if *load_balance_hosts {
                    uri.push_str("&load_balance_hosts=true");
                }

                if keepalives == &Some(true) {
                    uri.push_str("&keepalives=1");
                    if let Some(v) = keepalives_idle {
                        write!(&mut uri, "&keepalives_idle={}", v.as_secs()).unwrap();
                    }
                    if let Some(v) = keepalives_interval {
                        write!(&mut uri, "&keepalives_interval={}", v.as_secs()).unwrap();
                    }
                    if let Some(v) = keepalives_count {
                        write!(&mut uri, "&keepalives_count={}", v).unwrap();
                    }
                }
            }
        };

        // SSL mode
        {
            uri.push_str("&sslmode=");
            let mode = match self.ssl.mode {
                SslMode::Disable => "disable",
                SslMode::Prefer => "prefer",
                SslMode::Require => "require",
                SslMode::VerifyCa => "verify-ca",
                SslMode::VerifyFull => "verify-full",
            };
            uri.push_str(mode);
        }
        if let Some(p) = &self.ssl.root_cert {
            write!(uri, "&sslrootcert={}", Encoded(p.as_str())).unwrap();
        }
        if let Some(p) = &self.ssl.client_cert {
            write!(uri, "&sslcert={}", Encoded(p.as_str())).unwrap();
        }
        if let Some(p) = &self.ssl.client_key {
            write!(uri, "&sslkey={}", Encoded(p.as_str())).unwrap();
        }
        if let Some(p) = &self.ssl.crl {
            write!(uri, "&sslcrl={}", Encoded(p.as_str())).unwrap();
        }
        if let Some(p) = &self.ssl.crl_dir {
            write!(uri, "&sslcrldir={}", Encoded(p.as_str())).unwrap();
        }

        if let Some(app) = &self.application_name {
            write!(uri, "&application_name={}", Encoded(app)).unwrap();
        }
        if let Some(t) = self.connect_timeout {
            write!(uri, "&connect_timeout={}", t.as_secs()).unwrap();
        }

        // Target Session Attributes
        {
            uri.push_str("&target_session_attrs=");
            let attr = match self.target_session_attrs {
                TargetSessionAttrs::Any => "any",
                TargetSessionAttrs::ReadWrite => "read-write",
                TargetSessionAttrs::ReadOnly => "read-only",
                TargetSessionAttrs::Primary => "primary",
                TargetSessionAttrs::Standby => "standby",
                TargetSessionAttrs::PreferStandby => "prefer-standby",
            };
            uri.push_str(attr);
        }

        for (key, value) in &self.extra_params {
            write!(uri, "&{}={}", Encoded(key), Encoded(value)).unwrap();
        }

        uri
    }

    fn write_userinfo(&self, uri: &mut String) {
        use std::fmt::Write as _;

        match (&self.user.is_empty(), &self.password) {
            (false, Some(pw)) => write!(
                uri,
                "{}:{}@",
                Encoded(&self.user),
                Encoded(pw.expose_secret())
            )
            .unwrap(),
            (false, None) => write!(uri, "{}@", Encoded(&self.user)).unwrap(),
            (true, _) => {}
        }
    }
}
