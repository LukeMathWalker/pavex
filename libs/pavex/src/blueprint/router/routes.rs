use pavex_bp_schema::Blueprint as BlueprintSchema;

/// The type returned by [`Blueprint::routes`].
///
/// It allows you to further configure the behaviour of the registered routes.
///
/// [`Blueprint::routes`]: crate::blueprint::Blueprint::routes
pub struct RegisteredRoutes<'a> {
    #[allow(unused)]
    pub(crate) blueprint: &'a mut BlueprintSchema,
    /// The index of the registered routes import in the blueprint's `components` vector.
    #[allow(unused)]
    pub(crate) component_id: usize,
}
