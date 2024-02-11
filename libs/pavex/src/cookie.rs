use crate::blueprint::constructor::{Constructor, Lifecycle, RegisteredConstructor};
use crate::blueprint::Blueprint;
use crate::f;
use crate::middleware::Next;
use crate::request::RequestHead;
use crate::response::Response;
use http::header::SET_COOKIE;
use http::HeaderValue;
use std::future::IntoFuture;
use std::sync::{Mutex, MutexGuard};

pub use cookie::Cookie;

#[derive(Debug)]
/// A wrapper around [CookieJar](https://docs.rs/cookie/0.18.0/cookie/struct.CookieJar.html) from the [cookie crate](https://docs.rs/cookie/0.18.0/cookie), that can be injected as a shared reference to conveniently manage client state.
/// It will encode and decode values to allow for reserved characters. Note that a simple cookie **can never be trusted** and **should never contain sensitive information**.
pub struct Cookies {
    jar: Box<Mutex<cookie::CookieJar>>,
}

impl Cookies {
    /// Builds a CookieJar from request headers.
    pub fn from_request_head(head: &RequestHead) -> Self {
        let mut jar = cookie::CookieJar::new();
        head.headers
            .get_all("cookie")
            .iter()
            .filter_map(|headervalue| headervalue.to_str().ok())
            .flat_map(|c| c.split("; "))
            .filter_map(|c| Cookie::parse_encoded(c.to_owned()).ok())
            .for_each(|c| jar.add_original(c));
        Cookies {
            jar: Box::from(Mutex::new(jar)),
        }
    }

    /// Retrieve a cookie by name.
    pub fn get(&self, name: &str) -> Option<Cookie> {
        self.inner().get(name).cloned()
    }

    /// Get all the currently registered cookies.
    /// This will also reflect changes that are not yet sent to the client.
    pub fn get_all(&self) -> Vec<Cookie> {
        self.inner().iter().map(|c| c.to_owned()).collect()
    }

    /// Add a cookie. If a cookie by the same name already exists, this will override its value.
    pub fn add(&self, cookie: Cookie<'static>) {
        self.inner().add(cookie)
    }

    /// Mark a cookie for deletion.
    pub fn remove(&self, cookie: Cookie<'static>) {
        self.inner().remove(cookie)
    }

    /// Attaches the changes in the Cookies to a Response
    /// This should only be done once per Response and is generally handled by cookies_middleware
    pub fn apply_delta(&self, mut response: Response) -> Response {
        for delta in self.inner().delta() {
            match HeaderValue::from_str(&delta.encoded().to_string()) {
                Ok(headervalue) => response = response.append_header(SET_COOKIE, headervalue),
                Err(e) => {
                    tracing::warn!(warning="Ignoring bad cookie.", context=%e);
                }
            }
        }
        response
    }

    fn inner(&self) -> MutexGuard<'_, cookie::CookieJar> {
        // anything that could cause the mutex to be poisoned will also cause the request to fail
        self.jar.lock().expect("Mutex should not be poisoned")
    }
}

/// Middleware to handle updating cookies
pub async fn cookies_middleware<C>(next: Next<C>, cookies: &Cookies) -> Response
where
    C: IntoFuture<Output = Response>,
{
    let response = next.await;
    cookies.apply_delta(response)
}

impl Cookies {
    /// Register the [default constructor](Self::default_constructor)
    /// for [`Cookies`] with a [`Blueprint`].
    pub fn register(bp: &mut Blueprint) -> RegisteredConstructor {
        Self::default_constructor().register(bp)
    }

    /// The [default constructor](Cookies::extract) for [`Cookies`].
    pub fn default_constructor() -> Constructor {
        Constructor::new(
            f!(pavex::cookie::Cookies::from_request_head),
            Lifecycle::RequestScoped,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::Cookies;
    use crate::{
        http::header::{COOKIE, SET_COOKIE},
        response::Response,
    };

    #[test]
    fn empty_cookie_jar() {
        let (parts, _) = http::Request::builder().body(()).unwrap().into_parts();
        let cookies = Cookies::from_request_head(&parts.into());
        assert!(cookies.get_all().is_empty());
    }

    #[test]
    fn add_and_remove_cookies() {
        let (parts, _) = http::Request::builder().body(()).unwrap().into_parts();
        let cookies = Cookies::from_request_head(&parts.into());
        cookies.add(("flavour", "chocolate chip").into());
        cookies.add(("foo", "bar").into());
        cookies.remove("foo".into());
        assert!(cookies.get("foo").is_none());
        assert_eq!(cookies.get("flavour").unwrap().value(), "chocolate chip");
    }

    #[test]
    fn extract_cookies_and_apply_delta() {
        let (parts, _) = http::Request::builder()
            .header(COOKIE, "flavour=yummy; foo=bar")
            .header(COOKIE, "baz=bam")
            .body(())
            .unwrap()
            .into_parts();
        let cookies = Cookies::from_request_head(&parts.into());
        assert_eq!(cookies.get("foo").unwrap().value(), "bar");
        assert_eq!(cookies.get("baz").unwrap().value(), "bam");
        let flavourcookie = cookies.get("flavour").unwrap();
        assert_eq!(flavourcookie.value(), "yummy");
        cookies.remove((flavourcookie.name().to_owned()).into());
        let mut res = Response::ok();
        res = cookies.apply_delta(res);
        let responsecookies = res
            .headers()
            .get_all(SET_COOKIE)
            .iter()
            .map(|hv| hv.to_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(responsecookies.len(), 1);
        assert!(responsecookies[0].starts_with("flavour="));
        assert!(responsecookies[0].contains("Max-Age=0"));
    }

    #[test]
    fn encode_and_decode() {
        let (parts, _) = http::Request::builder()
            .header(
                COOKIE,
                "test=%20%22%23%25%26%28%29%2c%2f%3a%3b%3d%3f%40%5b%5d",
            )
            .body(())
            .unwrap()
            .into_parts();
        let cookies = Cookies::from_request_head(&parts.into());
        let testv = cookies.get("test").unwrap().value().to_owned();
        assert_eq!(testv, " \"#%&(),/:;=?@[]");
        cookies.add(("test2", testv).into());
        let mut res = Response::ok();
        res = cookies.apply_delta(res);
        let responsecookies = res
            .headers()
            .get_all(SET_COOKIE)
            .iter()
            .map(|hv| hv.to_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(responsecookies.len(), 1);
        assert!(
            responsecookies[0].starts_with("test2=%20%22%23%25&%28%29%2C%2F%3A%3B%3D%3F%40%5B%5D")
        );
    }
}
