use pavex_bp_schema::Blueprint as BlueprintSchema;

use super::reflection::{CreatedAt, Sources};

/// The input type for [`Blueprint::import`] and [`Blueprint::routes`].
///
/// A list of modules you want to import components from.
///
/// # Stability
///
/// [`Import`] is always populated by the [`from!`] macro.
/// Newer versions of Pavex may introduce, remove or modify the fields of this type—it is considered
/// an implementation detail of [`from!`] macros and should not be used directly.
///
/// Invoke the [`from!`] macro wherever an instance of [`Import`] is needed.
///
/// [`from!`]: super::from
/// [`Blueprint::import`]: crate::Blueprint::import
/// [`Blueprint::routes`]: crate::Blueprint::routes
pub struct Import {
    /// The modules you want to import components from.
    pub sources: Sources,
    #[doc(hidden)]
    /// The location where this instance of [`Import`] was created, invoking the [`from!`] macro.
    ///
    /// [`from!`]: super::from
    pub created_at: CreatedAt,
}

/// The type returned by [`Blueprint::import`].
///
/// It allows you to further configure the behaviour of the registered import.
///
/// [`Blueprint::import`]: crate::Blueprint::import
pub struct RegisteredImport<'a> {
    #[allow(unused)]
    pub(crate) blueprint: &'a mut BlueprintSchema,
    /// The index of the registered import in the blueprint's `components` vector.
    #[allow(unused)]
    pub(crate) component_id: usize,
}
