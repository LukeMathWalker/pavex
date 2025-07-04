use pavex_bp_schema::{Blueprint as BlueprintSchema, Component, PrebuiltType};

use crate::blueprint::{CloningPolicy, conversions::cloning2cloning};

use super::reflection::AnnotationCoordinates;

/// The input type for [`Blueprint::prebuilt`].
///
/// Check out [`Blueprint::prebuilt`] for more information on prebuilt types
/// in Pavex.
///
/// # Stability guarantees
///
/// Use the [`prebuilt`](macro@crate::prebuilt) attribute macro to create instances of `Prebuilt`.\
/// `Prebuilt`'s fields are an implementation detail of Pavex's macros and should not be relied upon:
/// newer versions of Pavex may add, remove or modify its fields.
///
/// [`Blueprint::prebuilt`]: crate::Blueprint::prebuilt
pub struct Prebuilt {
    #[doc(hidden)]
    pub coordinates: AnnotationCoordinates,
}

/// The type returned by [`Blueprint::prebuilt`].
///
/// It allows you to further configure the behaviour of the registered prebuilt type.
///
/// [`Blueprint::prebuilt`]: crate::Blueprint::prebuilt
pub struct RegisteredPrebuilt<'a> {
    #[allow(unused)]
    pub(crate) blueprint: &'a mut BlueprintSchema,
    /// The index of the registered prebuilt type in the blueprint's `components` vector.
    #[allow(unused)]
    pub(crate) component_id: usize,
}

impl RegisteredPrebuilt<'_> {
    /// Set the cloning strategy for the output type returned by this constructor.
    ///
    /// By default,
    /// Pavex will **never** try to clone a prebuilt type.
    /// If the type implements [`Clone`], you can change the default by invoking
    /// [`clone_if_necessary`](Self::clone_if_necessary): Pavex will clone the prebuilt type if
    /// it's necessary to generate code that satisfies Rust's borrow checker.
    pub fn cloning(mut self, strategy: CloningPolicy) -> Self {
        self.prebuilt().cloning_policy = Some(cloning2cloning(strategy));
        self
    }

    /// Set the cloning strategy to [`CloningPolicy::CloneIfNecessary`].
    /// Check out [`cloning`](Self::cloning) method for more details.
    pub fn clone_if_necessary(self) -> Self {
        self.cloning(CloningPolicy::CloneIfNecessary)
    }

    /// Set the cloning strategy to [`CloningPolicy::NeverClone`].
    /// Check out [`cloning`](Self::cloning) method for more details.
    pub fn never_clone(self) -> Self {
        self.cloning(CloningPolicy::NeverClone)
    }

    fn prebuilt(&mut self) -> &mut PrebuiltType {
        let component = &mut self.blueprint.components[self.component_id];
        let Component::PrebuiltType(s) = component else {
            unreachable!("The component should be a prebuilt type")
        };
        s
    }
}
