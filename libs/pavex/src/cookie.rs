use crate::blueprint::constructor::{
    CloningStrategy, Constructor, Lifecycle, RegisteredConstructor,
};
use crate::blueprint::Blueprint;
use crate::f;
use crate::middleware::Next;
use crate::request::RequestHead;
use crate::response::Response;
use cookie::CookieJar;
use http::header::SET_COOKIE;
use http::HeaderValue;
use std::cell::RefCell;
use std::future::IntoFuture;
use std::rc::Rc;

pub use cookie::Cookie;

#[derive(Debug)]
/// Structure that facilitates reading Cookies from HTTP Request Headers
/// Wraps a [`CookieJar`] from the cookie crate.  
pub struct RequestCookies {
    jar: CookieJar,
}

impl RequestCookies {
    /// Extracts [`Cookie`]s from a [`RequestHead`] and adds them to a new [`CookieJar`]
    /// Decodes percent-encoded characters.
    /// Silently skips invalid Headers and Cookies.
    /// Accepts multiple `Cookie`-Headers.
    /// If there are multiple cookies by the same name present, the last one wins.
    pub fn extract(head: &RequestHead) -> Self {
        let mut jar = cookie::CookieJar::new();
        head.headers
            .get_all("cookie")
            .iter()
            .filter_map(|headervalue| headervalue.to_str().ok())
            .flat_map(|c| c.split("; "))
            .filter_map(|c| Cookie::parse_encoded(c.to_owned()).ok())
            .for_each(|c| jar.add_original(c));
        RequestCookies { jar }
    }

    /// Get a reference to a [`Cookie`] by name, if it exists.
    pub fn get(&self, name: &str) -> Option<&Cookie> {
        self.jar.get(name)
    }

    /// Get a reference to the underlying [`CookieJar`]
    pub fn jar(&self) -> &CookieJar {
        &self.jar
    }

    /// Iterator over all the [`Cookie`]s extracted from a Request
    pub fn iter(&self) -> impl Iterator<Item = &Cookie<'static>> {
        self.jar.iter()
    }
}

#[derive(Debug, Clone)]
pub struct ResponseCookies {
    jar: Rc<RefCell<CookieJar>>,
}
impl ResponseCookies {
    /// Creates an instance of [`ResponseCookies`] from [`RequestCookies`]
    pub fn from_request_cookies(request_cookies: &RequestCookies) -> Self {
        ResponseCookies {
            jar: Rc::new(RefCell::from(request_cookies.jar().to_owned())),
        }
    }

    /// Adds a [`Cookie`] to the [`CookieJar`]`.
    /// If a cookie by the same name is already present, it will be overwritten.
    pub fn add(&self, cookie: Cookie<'static>) {
        self.jar.borrow_mut().add(cookie)
    }

    /// Removes a [`Cookie`] from the [`CookieJar`]`.
    pub fn remove(&self, cookie: Cookie<'static>) {
        self.jar.borrow_mut().remove(cookie)
    }

    /// Gets a Copy of a [`Cookie`] by name, if it exists.
    pub fn get_cloned(&self, key: &str) -> Option<Cookie> {
        self.jar.borrow().get(key).map(|c| c.to_owned())
    }

    /// Gets a Copy of all the [`Cookie`]s present in the [`CookieJar`].
    pub fn get_all_cloned(&self) -> Vec<Cookie> {
        self.jar.borrow().iter().map(|c| c.to_owned()).collect()
    }

    /// Attaches the changes in the [`CookieJar`] to the [`Response`]
    /// It percent-encodes reserved characters.
    /// This should only be done once per Response and is generally handled by [cookie_middleware](cookie_middleware)
    pub fn apply_delta(&self, mut response: Response) -> Response {
        for delta in self.jar.borrow().delta() {
            match HeaderValue::from_str(&delta.encoded().to_string()) {
                Ok(headervalue) => response = response.append_header(SET_COOKIE, headervalue),
                Err(e) => {
                    tracing::warn!(warning="Ignoring bad cookie.", context=%e);
                }
            }
        }
        response
    }
}

/// Middleware to handle updating cookies
pub async fn cookie_middleware<C>(next: Next<C>, cookies: &ResponseCookies) -> Response
where
    C: IntoFuture<Output = Response>,
{
    let response = next.await;
    cookies.apply_delta(response)
}

impl RequestCookies {
    /// Register the [default constructor](Self::default_constructor)
    /// for [`RequestCookies`] with a [`Blueprint`].
    pub fn register(bp: &mut Blueprint) -> RegisteredConstructor {
        Self::default_constructor().register(bp)
    }

    /// The [default constructor](RequestCookies::extract) for [`RequestCookies`].
    pub fn default_constructor() -> Constructor {
        Constructor::new(
            f!(crate::cookie::RequestCookies::extract),
            Lifecycle::RequestScoped,
        )
    }
}

impl ResponseCookies {
    /// Register the [default constructor](Self::default_constructor)
    /// for [`ResponseCookies`] with a [`Blueprint`].
    pub fn register(bp: &mut Blueprint) -> RegisteredConstructor {
        Self::default_constructor().register(bp)
    }

    /// The [default constructor](ResponseCookies::from_request_cookies) for [`ResponseCookies`].
    pub fn default_constructor() -> Constructor {
        Constructor::new(
            f!(crate::cookie::ResponseCookies::from_request_cookies),
            Lifecycle::RequestScoped,
        )
        .cloning(CloningStrategy::CloneIfNecessary)
    }
}

#[cfg(test)]
mod tests {
    use super::{RequestCookies, ResponseCookies};
    use crate::{
        http::header::{COOKIE, SET_COOKIE},
        response::Response,
    };

    #[test]
    fn empty_cookie_jar() {
        let (parts, _) = http::Request::builder().body(()).unwrap().into_parts();
        let cookies = RequestCookies::extract(&parts.into());
        assert!(cookies.iter().next().is_none());
    }

    #[test]
    fn add_and_remove_cookies() {
        let (parts, _) = http::Request::builder().body(()).unwrap().into_parts();
        let rcookies = RequestCookies::extract(&parts.into());
        let cookies = ResponseCookies::from_request_cookies(&rcookies);
        cookies.add(("flavour", "chocolate chip").into());
        cookies.add(("foo", "bar").into());
        cookies.remove("foo".into());
        assert!(cookies.get_cloned("foo").is_none());
        assert_eq!(
            cookies.get_cloned("flavour").unwrap().value(),
            "chocolate chip"
        );
    }

    #[test]
    fn extract_cookies_and_apply_delta() {
        let (parts, _) = http::Request::builder()
            .header(COOKIE, "flavour=yummy; foo=bar")
            .header(COOKIE, "baz=bam")
            .body(())
            .unwrap()
            .into_parts();
        let cookies = RequestCookies::extract(&parts.into());
        assert_eq!(cookies.get("foo").unwrap().value(), "bar");
        assert_eq!(cookies.get("baz").unwrap().value(), "bam");
        let flavourcookie = cookies.get("flavour").unwrap();
        assert_eq!(flavourcookie.value(), "yummy");
        let cookies = ResponseCookies::from_request_cookies(&cookies);
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
        let cookies = RequestCookies::extract(&parts.into());
        let testv = cookies.get("test").unwrap().value().to_owned();
        assert_eq!(testv, " \"#%&(),/:;=?@[]");
        let cookies = ResponseCookies::from_request_cookies(&cookies);
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
