#[derive(Debug, Copy, Clone, PartialEq, Eq, Ord, PartialOrd)]
pub(crate) enum QueueItem {
    /// The `id` of an enum, struct, trait or function.
    Standalone(rustdoc_types::Id),
    Impl {
        /// The `id` of the `Self` type for this `impl` block.
        self_: rustdoc_types::Id,
        /// The `id` of the `impl` block item.
        id: rustdoc_types::Id,
    },
    ImplItem {
        /// The `id` of the `Self` type for this `impl` block.
        self_: rustdoc_types::Id,
        /// The `id` of the `impl` block that this item belongs to.
        impl_: rustdoc_types::Id,
        /// The `id` of the `impl` block item.
        id: rustdoc_types::Id,
    },
}
