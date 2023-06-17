use ahash::{HashMap, HashMapExt};
use bimap::BiHashMap;
use guppy::graph::PackageGraph;
use pavex::blueprint::constructor::{CloningStrategy, Lifecycle};
use proc_macro2::Ident;
use quote::format_ident;

use crate::{
    compiler::utils::process_framework_path, language::ResolvedType, rustdoc::CrateCollection,
};

/// The id for a framework item inside [`FrameworkItemDb`].
pub(crate) type FrameworkItemId = u8;

/// The set of types that are built into the framework—e.g. the incoming request,
/// raw route parameters.
///
/// These types can be used by constructors and handlers even though no constructor
/// has been explicitly registered for them by the developer.
pub(crate) struct FrameworkItemDb {
    items: HashMap<ResolvedType, FrameworkItemId>,
    id2metadata: HashMap<FrameworkItemId, FrameworkItemMetadata>,
}

impl FrameworkItemDb {
    /// Initialise the database of framework items.
    ///
    /// The list is currently hard-coded, but we can imagine a future where it becomes configurable
    /// (e.g. if we want to reuse the DI machinery for more than a single web framework).
    #[tracing::instrument("Build framework items database", skip_all)]
    pub fn new(package_graph: &PackageGraph, krate_collection: &CrateCollection) -> Self {
        let capacity = 2;
        let mut items = HashMap::with_capacity(capacity);
        let mut id2metadata = HashMap::with_capacity(capacity);

        let request_head = process_framework_path(
            "pavex::request::RequestHead",
            package_graph,
            krate_collection,
        );
        items.insert(request_head, 0);
        id2metadata.insert(
            0,
            FrameworkItemMetadata {
                lifecycle: Lifecycle::RequestScoped,
                cloning_strategy: CloningStrategy::NeverClone,
                binding: format_ident!("request_head"),
            },
        );
        let http_request =
            process_framework_path("pavex::hyper::Body", package_graph, krate_collection);
        items.insert(http_request, 1);
        id2metadata.insert(
            1,
            FrameworkItemMetadata {
                lifecycle: Lifecycle::RequestScoped,
                cloning_strategy: CloningStrategy::NeverClone,
                binding: format_ident!("request_body"),
            },
        );
        let raw_path_parameters = process_framework_path(
            "pavex::extract::route::RawRouteParams::<'server, 'request>",
            package_graph,
            krate_collection,
        );
        items.insert(raw_path_parameters, 2);
        id2metadata.insert(
            2,
            FrameworkItemMetadata {
                lifecycle: Lifecycle::RequestScoped,
                cloning_strategy: CloningStrategy::CloneIfNecessary,
                binding: format_ident!("url_params"),
            },
        );
        Self { items, id2metadata }
    }

    /// Return the [`Lifecycle`] associated with a framework item.
    pub(crate) fn lifecycle(&self, item_id: FrameworkItemId) -> Lifecycle {
        self.id2metadata[&item_id].lifecycle
    }

    /// Return the [`CloningStrategy`] associated with a framework item.
    pub(crate) fn cloning_strategy(&self, item_id: FrameworkItemId) -> CloningStrategy {
        self.id2metadata[&item_id].cloning_strategy
    }

    /// Return the [`FrameworkItemId`] for a type, if it's a framework item.
    /// `None` otherwise.
    pub(crate) fn get_id(&self, type_: &ResolvedType) -> Option<FrameworkItemId> {
        self.items.get(type_).copied()
    }

    /// Return a bijective map that associates each framework type with an identifier (i.e. a variable name).
    ///
    /// This is used for code-generation.
    pub(crate) fn bindings(&self) -> BiHashMap<Ident, ResolvedType> {
        self.items
            .iter()
            .map(|(type_, id)| (self.id2metadata[id].binding.clone(), type_.to_owned()))
            .collect()
    }

    /// Iterate over all the items in the database alongside their ids.
    pub fn iter(
        &self,
    ) -> impl Iterator<Item = (FrameworkItemId, &ResolvedType)> + ExactSizeIterator {
        self.items.iter().map(|(t, id)| (*id, t))
    }
}

struct FrameworkItemMetadata {
    lifecycle: Lifecycle,
    binding: Ident,
    cloning_strategy: CloningStrategy,
}
