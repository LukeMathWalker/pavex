use pavex::cookie::SameSite;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
/// Configure the cookie used to store session information on the client-side.
pub struct SessionCookieConfig {
    /// The name of the cookie used to store the session ID.
    ///
    /// By default, the name is set to `id`.
    #[serde(default = "default_session_cookie_name")]
    pub name: String,
    /// Set the `Domain` attribute on the cookie used to store the session ID.
    ///
    /// By default, the attribute is not set.
    #[serde(default)]
    pub domain: Option<String>,
    /// Set the `Path` attribute on the cookie used to store the session ID.
    ///
    /// By default, the attribute is set to `/`.
    #[serde(default = "default_session_cookie_path")]
    pub path: Option<String>,
    /// Set the `Secure` attribute on the cookie used to store the session ID.
    ///
    /// If the cookie is marked as `Secure`, it will only be transmitted when the connection is secure (e.g. over HTTPS).
    ///
    /// Default is `true`.
    #[serde(default = "default_session_cookie_secure")]
    pub secure: bool,
    /// Set the `HttpOnly` attribute on the cookie used to store the session ID.
    ///
    /// If the cookie is marked as `HttpOnly`, it will not be visible to JavaScript
    /// snippets running in the browser.
    ///
    /// Default is `true`.
    #[serde(default = "default_session_cookie_http_only")]
    pub http_only: bool,
    /// Set the [`SameSite`] attribute on the cookie used to store the session ID.
    ///
    /// By default, the attribute is set to [`SameSite::Lax`].
    #[serde(default = "default_session_cookie_same_site")]
    #[serde(with = "same_site")]
    pub same_site: Option<SameSite>,
    /// The kind of session cookie to use.
    ///
    /// By default, it is set to [`SessionCookieKind::Persistent`].
    #[serde(default)]
    pub kind: SessionCookieKind,
}

impl Default for SessionCookieConfig {
    fn default() -> Self {
        Self {
            name: default_session_cookie_name(),
            domain: None,
            path: default_session_cookie_path(),
            secure: default_session_cookie_secure(),
            http_only: default_session_cookie_http_only(),
            same_site: default_session_cookie_same_site(),
            kind: Default::default(),
        }
    }
}

fn default_session_cookie_name() -> String {
    // See https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html#session-id-name-fingerprinting
    "id".to_string()
}

fn default_session_cookie_secure() -> bool {
    true
}

fn default_session_cookie_http_only() -> bool {
    true
}

fn default_session_cookie_path() -> Option<String> {
    Some("/".to_string())
}

fn default_session_cookie_same_site() -> Option<SameSite> {
    Some(SameSite::Lax)
}

/// The kind of cookie used to store session information on the client-side.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SessionCookieKind {
    /// A persistent session cookie.
    ///
    /// The cookie will be stored on the client's device with an
    /// expiration date set by the server via the `Max-Age` attribute.
    ///
    /// This is the default.
    #[default]
    Persistent,
    /// A cookie that expires when the browser session ends.
    ///
    /// Each browser has its own concept of "browser session", e.g. the session
    /// doesn't necessarily end when the browser window or tab is closed.
    /// For example, both Firefox and Chrome automatically restore the session
    /// when the browser is restarted, keeping all session cookies alive.   
    /// Consider using [`SessionCookieKind::Persistent`]
    /// if you don't want to deal with the nuances of browser-specific behaviour.
    Session,
}

// Deserialization and serialization routines for `same_site` attribute.
mod same_site {
    use pavex::cookie::SameSite;
    use serde::{de, Deserializer, Serializer};
    use std::fmt;

    pub fn serialize<S>(value: &Option<SameSite>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(same_site) => {
                let same_site = match same_site {
                    SameSite::Strict => "Strict",
                    SameSite::Lax => "Lax",
                    SameSite::None => "None",
                };
                serializer.serialize_some(same_site)
            }
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<SameSite>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct SameSiteVisitor;

        impl<'de> de::Visitor<'de> for SameSiteVisitor {
            type Value = Option<SameSite>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string or null")
            }

            fn visit_str<E>(self, value: &str) -> Result<Option<SameSite>, E>
            where
                E: de::Error,
            {
                match value {
                    "Strict" | "strict" => Ok(Some(SameSite::Strict)),
                    "Lax" | "lax" => Ok(Some(SameSite::Lax)),
                    "None" | "none" => Ok(Some(SameSite::None)),
                    _ => Err(de::Error::unknown_variant(
                        value,
                        &["Strict", "Lax", "None"],
                    )),
                }
            }

            fn visit_none<E>(self) -> Result<Option<SameSite>, E>
            where
                E: de::Error,
            {
                Ok(None)
            }
        }

        deserializer.deserialize_option(SameSiteVisitor)
    }
}
