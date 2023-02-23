/// A marker trait implemented for all "callable" types in Rust - e.g. free functions, methods.
///
/// # What is the purpose of `Callable`?
///
/// `pavex`'s constructors and request handlers can accept an arbitrary number of input
/// parameters.  
/// How do we encode this requirement in the signature of [`Blueprint`](crate::Blueprint)'s
/// methods?
///
/// We rely on `Callable`:
///
/// ```
/// use pavex_builder::Callable;
///
/// // `callable` can have an arbitrary number of input parameters.
/// fn f<CallableType, Inputs>(callable: CallableType)
/// where
///     CallableType: Callable<Inputs>
/// {
///     // [...]
/// }
///
/// // They all compile!
/// f(parameterless_function);
/// f(one_parameter_function);
/// f(A::static_method);
/// f(A::non_static_method);
///
/// fn parameterless_function() {}
/// fn one_parameter_function(a: usize) -> u8 { todo!() }
///
/// struct A;
/// impl A {
///     fn static_method() -> u16 { todo!() }
///     fn non_static_method(&self) -> u16 { todo!() }
/// }
/// ```
pub trait Callable<Inputs> {
    /// The output type returned by the callable when invoked.
    type Output;
}

macro_rules! impl_callable {
    ($($var:ident),*) => {
        impl <$($var,)* OutputType, FunctionType> Callable<($($var,)*)> for FunctionType
        where
            FunctionType: Fn($($var,)*) -> OutputType
        {
            type Output = OutputType;
        }
    };
}

impl_callable!();
impl_callable!(A);
impl_callable!(A, B);
impl_callable!(A, B, C);
impl_callable!(A, B, C, D);
impl_callable!(A, B, C, D, E);
impl_callable!(A, B, C, D, E, F);
impl_callable!(A, B, C, D, E, F, G);
impl_callable!(A, B, C, D, E, F, G, H);
impl_callable!(A, B, C, D, E, F, G, H, I);
impl_callable!(A, B, C, D, E, F, G, H, I, J);
impl_callable!(A, B, C, D, E, F, G, H, I, J, K);
