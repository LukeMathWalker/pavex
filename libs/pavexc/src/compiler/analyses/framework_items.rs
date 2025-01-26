use ahash::{HashMap, HashMapExt};
use bimap::BiHashMap;
use proc_macro2::Ident;
use quote::format_ident;

use crate::{
    compiler::utils::process_framework_path, language::ResolvedType, rustdoc::CrateCollection,
};
use pavex_bp_schema::{CloningStrategy, Lifecycle};

/// The id for a framework item inside [`FrameworkItemDb`].
pub(crate) type FrameworkItemId = u8;

/// The set of types that are built into the framework—e.g. the incoming request,
/// raw path parameters.
///
/// These types can be used by constructors and handlers even though no constructor
/// has been explicitly registered for them by the developer.
pub(crate) struct FrameworkItemDb {
    items: BiHashMap<ResolvedType, FrameworkItemId>,
    id2metadata: HashMap<FrameworkItemId, FrameworkItemMetadata>,
}

impl FrameworkItemDb {
    /// Initialise the database of framework items.
    ///
    /// The list is currently hard-coded, but we can imagine a future where it becomes configurable
    /// (e.g. if we want to reuse the DI machinery for more than a single web framework).
    #[tracing::instrument("Build framework items database", skip_all)]
    pub fn new(krate_collection: &CrateCollection) -> Self {
        let capacity = 2;
        let mut items = BiHashMap::with_capacity(capacity);
        let mut id2metadata = HashMap::with_capacity(capacity);

        let request_head = process_framework_path("pavex::request::RequestHead", krate_collection);
        items.insert(request_head, Self::request_head_id());
        id2metadata.insert(
            Self::request_head_id(),
            FrameworkItemMetadata {
                lifecycle: Lifecycle::RequestScoped,
                cloning_strategy: CloningStrategy::NeverClone,
                binding: format_ident!("request_head"),
            },
        );
        let http_request =
            process_framework_path("pavex::request::body::RawIncomingBody", krate_collection);
        items.insert(http_request, Self::raw_incoming_body_id());
        id2metadata.insert(
            Self::raw_incoming_body_id(),
            FrameworkItemMetadata {
                lifecycle: Lifecycle::RequestScoped,
                cloning_strategy: CloningStrategy::NeverClone,
                binding: format_ident!("request_body"),
            },
        );
        let raw_path_parameters = process_framework_path(
            "pavex::request::path::RawPathParams::<'server, 'request>",
            krate_collection,
        );
        items.insert(raw_path_parameters, Self::url_params_id());
        id2metadata.insert(
            Self::url_params_id(),
            FrameworkItemMetadata {
                lifecycle: Lifecycle::RequestScoped,
                cloning_strategy: CloningStrategy::CloneIfNecessary,
                binding: format_ident!("url_params"),
            },
        );
        let matched_route_template =
            process_framework_path("pavex::request::path::MatchedPathPattern", krate_collection);
        items.insert(matched_route_template, Self::matched_route_template_id());
        id2metadata.insert(
            Self::matched_route_template_id(),
            FrameworkItemMetadata {
                lifecycle: Lifecycle::RequestScoped,
                cloning_strategy: CloningStrategy::CloneIfNecessary,
                binding: format_ident!("matched_route_template"),
            },
        );

        let allowed_methods =
            process_framework_path("pavex::router::AllowedMethods", krate_collection);
        items.insert(allowed_methods, Self::allowed_methods_id());
        id2metadata.insert(
            Self::allowed_methods_id(),
            FrameworkItemMetadata {
                lifecycle: Lifecycle::RequestScoped,
                cloning_strategy: CloningStrategy::CloneIfNecessary,
                binding: format_ident!("allowed_methods"),
            },
        );

        let connection_info =
            process_framework_path("pavex::connection::ConnectionInfo", krate_collection);
        items.insert(connection_info, Self::connection_info_id());
        id2metadata.insert(
            Self::connection_info_id(),
            FrameworkItemMetadata {
                lifecycle: Lifecycle::RequestScoped,
                cloning_strategy: CloningStrategy::CloneIfNecessary,
                binding: format_ident!("connection_info"),
            },
        );
        Self { items, id2metadata }
    }

    /// Return the id for the `RequestHead` type.
    pub(crate) fn request_head_id() -> FrameworkItemId {
        0
    }

    /// Return the id for the `RawIncomingBody` type.
    pub(crate) fn raw_incoming_body_id() -> FrameworkItemId {
        1
    }

    /// Return the id for the `PathParams` type.
    pub(crate) fn url_params_id() -> FrameworkItemId {
        2
    }

    /// Return the id for the `MatchedPathPattern` type.
    pub(crate) fn matched_route_template_id() -> FrameworkItemId {
        3
    }

    /// Return the id for the `AllowedMethods` type.
    pub(crate) fn allowed_methods_id() -> FrameworkItemId {
        4
    }

    /// Return the id for the `ConnectionInfo` type.
    pub(crate) fn connection_info_id() -> FrameworkItemId {
        5
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
        self.items.get_by_left(type_).copied()
    }

    /// Return the [`ResolvedType`] attached to a given [`FrameworkItemId`].
    pub(crate) fn get_type(&self, id: FrameworkItemId) -> &ResolvedType {
        self.items.get_by_right(&id).unwrap()
    }

    /// Return the binding associated with a given [`FrameworkItemId`].
    pub(crate) fn get_binding(&self, id: FrameworkItemId) -> &Ident {
        &self.id2metadata[&id].binding
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
    pub fn iter(&self) -> impl ExactSizeIterator<Item = (FrameworkItemId, &ResolvedType)> {
        self.items.iter().map(|(t, id)| (*id, t))
    }
}

struct FrameworkItemMetadata {
    lifecycle: Lifecycle,
    binding: Ident,
    cloning_strategy: CloningStrategy,
}
