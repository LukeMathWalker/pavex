use pavex_bp_schema::Blueprint as BlueprintSchema;

/// The type returned by [`Blueprint::state_input`].
///
/// It allows you to further configure the behaviour of the registered constructor.
///
/// [`Blueprint::state_input`]: crate::blueprint::Blueprint::state_input
pub struct RegisteredStateInput<'a> {
    #[allow(unused)]
    pub(crate) blueprint: &'a mut BlueprintSchema,
    /// The index of the registered state input in the blueprint's `components` vector.
    #[allow(unused)]
    pub(crate) component_id: usize,
}
