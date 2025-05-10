#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(super) enum QueueItem {
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

/// A lot of unnecessary jumping through hoops to implement `Ord`/`PartialOrd`
/// since `rustdoc_types::Id` doesn't implement `Ord`/`PartialOrd`.
mod sortable_queue {
    use super::QueueItem;
    use crate::compiler::analyses::user_components::annotations::sortable::SortableId;

    impl QueueItem {
        fn as_sortable(&self) -> (SortableId, Option<SortableId>, Option<SortableId>) {
            match self {
                QueueItem::Standalone(id) => ((*id).into(), None, None),
                QueueItem::Impl { self_, id } => ((*self_).into(), Some((*id).into()), None),
                QueueItem::ImplItem { self_, impl_, id } => {
                    ((*self_).into(), Some((*impl_).into()), Some((*id).into()))
                }
            }
        }
    }

    impl PartialOrd for QueueItem {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }

    impl Ord for QueueItem {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            let sortable_self = self.as_sortable();
            let sortable_other = other.as_sortable();
            sortable_self.cmp(&sortable_other)
        }
    }
}
