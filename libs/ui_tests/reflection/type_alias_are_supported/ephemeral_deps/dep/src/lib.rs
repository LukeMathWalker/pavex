use pavex::blueprint::{constructor::Lifecycle, Blueprint};
use pavex::f;

pub type IntermediateAlias = ActualType;
pub type IntermediateGenericAlias<A, B> = GenericType<A, B>;

pub struct DoubleLifetimeType<'a, 'b> {
    _a: &'a str,
    _b: &'b str,
}

impl<'a, 'b> DoubleLifetimeType<'a, 'b> {
    pub fn new(_t1: &'a ActualType, _t2: &'b String) -> DoubleLifetimeType<'a, 'b> {
        todo!()
    }
}

#[derive(Clone)]
pub struct ActualType;

impl ActualType {
    pub fn new() -> Self {
        todo!()
    }
}

#[derive(Clone)]
pub struct GenericType<A, B> {
    _a: A,
    _b: B,
}

// The naming of the generic parameters on this `impl` block is intentionally
// chosen to be different from the generic parameters on the struct definition
// to test Pavex's ability to handle this case.
impl<C, D> GenericType<C, D> {
    pub fn new() -> GenericType<C, D> {
        todo!()
    }
}
