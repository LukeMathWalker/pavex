use guppy::PackageId;

/// An identifier that unequivocally points to a type within a crate collection.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GlobalItemId {
    pub rustdoc_item_id: rustdoc_types::Id,
    pub package_id: PackageId,
}

impl GlobalItemId {
    pub fn new(rustdoc_item_id: rustdoc_types::Id, package_id: PackageId) -> Self {
        Self {
            rustdoc_item_id,
            package_id,
        }
    }

    pub fn package_id(&self) -> &PackageId {
        &self.package_id
    }

    pub fn rustdoc_item_id(&self) -> &rustdoc_types::Id {
        &self.rustdoc_item_id
    }
}
