use pavex_bp_schema::Blueprint as BlueprintSchema;

/// The type returned by [`Blueprint::routes`].
///
/// # Future-proofing
///
/// As of today, [`RegisteredRoutes`] doesn't provide any additional functionality.\
/// It is included to allow introducing new configuration for route groups without having
/// to change the signature of [`Blueprint::routes`].
///
/// [`Blueprint::routes`]: crate::Blueprint::routes
pub struct RegisteredRoutes<'a> {
    #[allow(unused)]
    pub(crate) blueprint: &'a mut BlueprintSchema,
    /// The index of the registered routes import in the blueprint's `components` vector.
    #[allow(unused)]
    pub(crate) component_id: usize,
}
