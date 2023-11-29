use crate::response::Response;

use super::AllowedMethods;

/// The default fallback handler for incoming requests that don't match
/// any of the routes you registered.
///
/// It returns a `404 Not Found` response if the path doesn't match any of the
/// registered route paths.  
/// It returns a `405 Method Not Allowed` response if the path matches a
/// registered route path but the method doesn't match any of its associated
/// handlers.
pub async fn default_fallback(allowed_methods: &AllowedMethods) -> Response {
    if allowed_methods.len() == 0 {
        Response::not_found().box_body()
    } else {
        let allow_header = join(
            &mut allowed_methods.iter().map(|method| method.as_str()),
            ",",
        );
        let allow_header =
            http::HeaderValue::from_str(&allow_header).expect("Invalid `Allow` header value");
        Response::method_not_allowed()
            .insert_header(http::header::ALLOW, allow_header)
            .box_body()
    }
}

// Inlined from `itertools to avoid adding a dependency.
fn join<'a, I>(iter: &mut I, separator: &str) -> String
where
    I: Iterator<Item = &'a str>,
{
    use std::fmt::Write;

    match iter.next() {
        None => String::new(),
        Some(first_elt) => {
            let mut result = String::with_capacity(separator.len() * iter.size_hint().0);
            write!(&mut result, "{}", first_elt).unwrap();
            iter.for_each(|element| {
                result.push_str(separator);
                write!(&mut result, "{}", element).unwrap();
            });
            result
        }
    }
}
