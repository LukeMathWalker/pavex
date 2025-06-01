use syn::{
    Attribute, Field, FieldValue, FnArg, ItemEnum, ItemImpl, ItemMod, ItemStruct, ItemTrait,
    TraitItem, Variant,
    visit_mut::{self, VisitMut},
};

/// A visitor that walks the entire AST and drops any `#[px(...)]` attribute.
///
/// This is necessary due to the fact that `proc_macro_attribute` doesn't have
/// a first-class notion of "macro helper attributes", unlike derive macros.
/// If our `#[px(...)]` helpers aren't stripped out, the compiler will try to
/// resolve `px` as if it were its own proc macro, triggering resolution errors.
///
/// There is an open issue to track when (or if) `proc_macro_attribute` will ever
/// provide built-in support for this: <https://github.com/rust-lang/rust/issues/65823>
///
/// Until then, we have to do it manually.
pub struct PxStripper;

impl VisitMut for PxStripper {
    /// Strip on any AST node that has `.attrs`.
    fn visit_attribute_mut(&mut self, _attr: &mut Attribute) {
        // we don’t do anything here—removal happens in the parent methods
    }

    fn visit_item_mod_mut(&mut self, node: &mut ItemMod) {
        node.attrs.retain(not_px_attr);
        visit_mut::visit_item_mod_mut(self, node);
    }

    fn visit_item_struct_mut(&mut self, node: &mut ItemStruct) {
        node.attrs.retain(not_px_attr);
        visit_mut::visit_item_struct_mut(self, node);
    }

    fn visit_item_enum_mut(&mut self, node: &mut ItemEnum) {
        node.attrs.retain(not_px_attr);
        visit_mut::visit_item_enum_mut(self, node);
    }

    fn visit_item_impl_mut(&mut self, node: &mut ItemImpl) {
        node.attrs.retain(not_px_attr);
        visit_mut::visit_item_impl_mut(self, node);
    }

    fn visit_item_trait_mut(&mut self, node: &mut ItemTrait) {
        node.attrs.retain(not_px_attr);
        visit_mut::visit_item_trait_mut(self, node);
    }

    fn visit_trait_item_mut(&mut self, node: &mut TraitItem) {
        visit_mut::visit_trait_item_mut(self, node);
    }

    fn visit_variant_mut(&mut self, node: &mut Variant) {
        node.attrs.retain(not_px_attr);
        visit_mut::visit_variant_mut(self, node);
    }

    fn visit_field_mut(&mut self, node: &mut Field) {
        node.attrs.retain(not_px_attr);
        visit_mut::visit_field_mut(self, node);
    }

    fn visit_field_value_mut(&mut self, node: &mut FieldValue) {
        node.attrs.retain(not_px_attr);
        visit_mut::visit_field_value_mut(self, node);
    }

    fn visit_fn_arg_mut(&mut self, node: &mut FnArg) {
        let attrs = match node {
            FnArg::Receiver(receiver) => &mut receiver.attrs,
            FnArg::Typed(pat_type) => &mut pat_type.attrs,
        };
        attrs.retain(not_px_attr);
        visit_mut::visit_fn_arg_mut(self, node);
    }
}

fn not_px_attr(a: &Attribute) -> bool {
    !a.path().is_ident("px")
}
