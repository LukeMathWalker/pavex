pub trait HasDefault {
    type Default;
}

impl<T> HasDefault for T {
    type Default = u8;
}

pub struct Container<T, D = <T as HasDefault>::Default>(pub std::marker::PhantomData<(T, D)>);
