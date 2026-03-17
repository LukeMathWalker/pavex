pub trait HasId {
    type Id;
}

pub struct User;

impl HasId for User {
    type Id = u64;
}

pub struct Keyed<T, K = <T as HasId>::Id>(pub std::marker::PhantomData<(T, K)>);
