//! # `tracing_log_error`
//!
//! A small utility crate to:
//!
//! - Capture all useful information when logging errors
//! - With a consistent naming and representation
pub mod fields;

/// A macro that desugars to an invocation of `tracing::error!` with all
/// error-related fields (the ones in [the `fields` module](crate::fields))
/// pre-populated.
///
/// # Basic invocation
///
/// ```rust
/// use tracing::{event, Level};
/// use tracing_log_error::{fields, log_error};
///
/// let e = std::io::Error::new(std::io::ErrorKind::Other, "My error");
/// // This 👇
/// log_error!(e, "The connection was dropped");
/// // is equivalent to this 👇
/// event!(
///     Level::ERROR,
///     {{ fields::ERROR_MESSAGE }} = fields::error_message(&e),
///     {{ fields::ERROR_DETAILS }} = fields::error_details(&e),
///     {{ fields::ERROR_SOURCE_CHAIN }} = fields::error_source_chain(&e),
///     "The connection was dropped"
/// );
/// ```
///
/// # Custom fields
///
/// You can add custom fields to the log event by prepending them ahead of the
/// error:
///
/// ```rust
/// use tracing::{event, Level};
/// use tracing_log_error::{fields, log_error};
///
/// let e = std::io::Error::new(std::io::ErrorKind::Other, "My error");
/// // This 👇
/// log_error!(e, custom_field = "value", "The connection was dropped");
/// // is equivalent to this 👇
/// event!(
///     Level::ERROR,
///     custom_field = "value",
///     {{ fields::ERROR_MESSAGE }} = fields::error_message(&e),
///     {{ fields::ERROR_DETAILS }} = fields::error_details(&e),
///     {{ fields::ERROR_SOURCE_CHAIN }} = fields::error_source_chain(&e),
///     "The connection was dropped"
/// );
/// ```
///
/// # Custom level
///
/// It may be useful, in some cases, to log an error at a level other than
/// `ERROR`. You can do this by specifying the level as a named argument:
///
/// ```rust
/// use tracing::{event, Level};
/// use tracing_log_error::{fields, log_error};
///
/// let e = std::io::Error::new(std::io::ErrorKind::Other, "My error");
/// // This 👇
/// log_error!(e, level: Level::WARN, "The connection was dropped");
/// // is equivalent to this 👇
/// event!(
///     Level::WARN,
///     {{ fields::ERROR_MESSAGE }} = fields::error_message(&e),
///     {{ fields::ERROR_DETAILS }} = fields::error_details(&e),
///     {{ fields::ERROR_SOURCE_CHAIN }} = fields::error_source_chain(&e),
///     "The connection was dropped"
/// );
/// ```
#[macro_export]
macro_rules! log_error {
    // ...
    ($err:expr, level: $lvl:expr, { $($fields:tt)* }) => (
        ::tracing::event!(
            $lvl,
            {{ $crate::fields::ERROR_MESSAGE }} = $crate::fields::error_message(&$err),
            {{ $crate::fields::ERROR_DETAILS }} = $crate::fields::error_details(&$err),
            {{ $crate::fields::ERROR_SOURCE_CHAIN }} = $crate::fields::error_source_chain(&$err),
            $($fields)*
        )
    );
    ($err:expr, level: $lvl:expr, { $($fields:tt)* }, $($arg:tt)+) => (
        ::tracing::event!(
            $lvl,
            {{ $crate::fields::ERROR_MESSAGE }} = $crate::fields::error_message(&$err),
            {{ $crate::fields::ERROR_DETAILS }} = $crate::fields::error_details(&$err),
            {{ $crate::fields::ERROR_SOURCE_CHAIN }} = $crate::fields::error_source_chain(&$err),
            { $($fields)* },
            $($arg)+
        )
    );
    ($err:expr, level: $lvl:expr, $($k:ident).+ = $($field:tt)*) => (
        $crate::log_error!(
            $err,
            level: $lvl,
            { $($k).+ = $($field)*}
        )
    );
    ($err:expr, level: $lvl:expr, ?$($k:ident).+, $($field:tt)*) => (
        $crate::log_error!(
            $err,
            level: $lvl,
            { ?$($k).+, $($field)*}
        )
    );
    ($err:expr, level: $lvl:expr, %$($k:ident).+, $($field:tt)*) => (
        $crate::log_error!(
            $err,
            level: $lvl,
            { %$($k).+, $($field)*}
        )
    );
    ($err:expr, level: $lvl:expr, ?$($k:ident).+) => (
        $crate::log_error!($err, level: $lvl, ?$($k).+,)
    );
    ($err:expr, level: $lvl:expr, %$($k:ident).+) => (
        $crate::log_error!($err, level: $lvl, %$($k).+,)
    );
    ($err:expr, level: $lvl:expr, $($k:ident).+) => (
        $crate::log_error!($err, level: $lvl, $($k).+,)
    );
    ($err:expr, level: $lvl:expr, $($arg:tt)*) => (
        $crate::log_error!($err, level: $lvl, { $($arg)* })
    );
    ($err:expr, level: $lvl:expr) => (
        $crate::log_error!($err, level: $lvl, { })
    );
    ($err:expr, { $($fields:tt)* }, $($arg:tt)+) => (
        $crate::log_error!($err, level: ::tracing::Level::ERROR, { $($fields)* }, $($arg)+)
    );
    ($err:expr, $($k:ident).+ = $($field:tt)*) => (
        $crate::log_error!(
            $err,
            level: ::tracing::Level::ERROR,
            $($k).+ = $($field)*
        )
    );
    ($err:expr, ?$($k:ident).+, $($field:tt)*) => (
        $crate::log_error!(
            $err,
            level: ::tracing::Level::ERROR,
            ?$($k).+,
            $($field)*
        )
    );
    ($err:expr, %$($k:ident).+, $($field:tt)*) => (
        $crate::log_error!(
            $err,
            level: ::tracing::Level::ERROR,
            $($field)*
        )
    );
    ($err:expr, ?$($k:ident).+) => (
        $crate::log_error!($err, level: ::tracing::Level::ERROR, ?$($k).+)
    );
    ($err:expr, %$($k:ident).+) => (
        $crate::log_error!($lvl, level: ::tracing::Level::ERROR, %$($k).+)
    );
    ($err:expr, $($k:ident).+) => (
        $crate::log_error!($err, level: ::tracing::Level::ERROR, $($k).+)
    );
    ($err:expr, $($arg:tt)*) => (
        $crate::log_error!($err, level: ::tracing::Level::ERROR, $($arg)*)
    );
    ($err:expr) => (
        $crate::log_error!($err,)
    );
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::log_error;

    #[test]
    fn my_test() {
        let e = std::io::Error::new(std::io::ErrorKind::Other, "My error");
        // Most common usage
        log_error!(e, "Yay");
        // Passing a reference to the error rather than an owned error
        log_error!(&e, "Yay");
        // Just the error
        log_error!(e);
        // Formatting in the message
        let a = "friend";
        log_error!(e, "Here I am, {}", a);
        log_error!(e, "Here I am, {} {}", "my", "friend");
        // Custom level
        log_error!(e, level: tracing::Level::WARN, "Yay");
        // Custom level, no message
        log_error!(e, level: tracing::Level::WARN);
        // Custom level, formatted message
        log_error!(e, level: tracing::Level::WARN, "Here I am, {}", a);
        // Custom fields
        log_error!(e, custom_field = "value", "Yay");
        // Custom fields with a message
        log_error!(e, custom_field1 = "value1", custom_field2 = "value2", "Yay");
        // Custom fields with a formatted message
        log_error!(
            e,
            custom_field1 = "value1",
            custom_field2 = "value2",
            "Here I am, {}",
            a
        );
        // Custom fields with a custom level
        log_error!(e, level: tracing::Level::INFO, custom_field1 = "value1", "Yay");
        // Custom fields with a custom level and formatted message
        log_error!(e, level: tracing::Level::INFO, custom_field1 = "value1", custom_field2 = "value2", "Here I am, {} {}", "my", "friend");
        // Using % and ? to log fields using their Display and Debug representations
        let a = PathBuf::from("a path");
        let b = "A string".to_string();
        log_error!(e, custom_field = ?a, custom_field2 = %b, ?a, %b, "Hello");
        // Using {{ }} to log fields using a constant as their name
        const FIELD: &str = "field";
        log_error!(
            e,
            {
                {
                    FIELD
                }
            } = "value",
            "Yay"
        );
    }
}
