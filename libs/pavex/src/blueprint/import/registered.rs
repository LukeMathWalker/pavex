use pavex_bp_schema::Blueprint as BlueprintSchema;

/// The type returned by [`Blueprint::import`].
///
/// It allows you to further configure the behaviour of the registered import.
///
/// [`Blueprint::import`]: crate::blueprint::Blueprint::import
pub struct RegisteredImport<'a> {
    #[allow(unused)]
    pub(crate) blueprint: &'a mut BlueprintSchema,
    /// The index of the registered import in the blueprint's `components` vector.
    #[allow(unused)]
    pub(crate) component_id: usize,
}
