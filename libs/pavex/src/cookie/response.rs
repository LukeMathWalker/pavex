//! Low-level types related to [`ResponseCookies`].
//!
//! [`ResponseCookies`]: super::ResponseCookies
use super::ResponseCookie;

/// Iterator over all the cookies in a [`ResponseCookies`].
///
/// [`ResponseCookies`]: super::ResponseCookies
pub struct ResponseCookiesIter<'map> {
    pub(crate) cookies: biscotti::response::ResponseCookiesIter<'map, 'static>,
}

impl<'map> Iterator for ResponseCookiesIter<'map> {
    type Item = &'map ResponseCookie<'static>;

    fn next(&mut self) -> Option<&'map ResponseCookie<'static>> {
        self.cookies.next()
    }
}
