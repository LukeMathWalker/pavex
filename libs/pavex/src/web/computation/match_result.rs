use crate::language::{GenericArgument, ResolvedType};

/// A branching constructor: extract one of the variant out of a Rust enum.
/// E.g. get a `T` (or `E`) from a `Result<T, E>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct MatchResult {
    pub(crate) input: ResolvedType,
    pub(crate) output: ResolvedType,
    pub(crate) variant: MatchResultVariant,
}

#[derive(Debug, Clone, PartialEq, Eq, Copy, Hash)]
pub(crate) enum MatchResultVariant {
    Ok,
    Err,
}

impl MatchResult {
    /// Return a new match-ing computation for the `Ok(T)` and the `Err(E)` variant of a `Result`.
    ///
    /// It panics if `result_type` is not an enum.
    pub(crate) fn match_result(result_type: &ResolvedType) -> ResultMatchers {
        let ResolvedType::ResolvedPath(inner_result_type) = result_type else {
            panic!("Expected a ResolvedPath, got {:?}", result_type)
        };
        assert_eq!(
            inner_result_type.generic_arguments.len(),
            2,
            "{result_type:?} does not have two generic arguments, as expected"
        );
        let mut generics = inner_result_type.generic_arguments.iter();
        let GenericArgument::AssignedTypeParameter(ok_type) = generics.next().unwrap().to_owned() else {
            unreachable!()
        };
        let ok_constructor = MatchResult {
            input: inner_result_type.clone().into(),
            output: ok_type,
            variant: MatchResultVariant::Ok,
        };
        let GenericArgument::AssignedTypeParameter(err_type) = generics.next().unwrap().to_owned() else {
            unreachable!()
        };
        let err_constructor = MatchResult {
            input: inner_result_type.clone().into(),
            output: err_type,
            variant: MatchResultVariant::Err,
        };
        ResultMatchers {
            ok: ok_constructor,
            err: err_constructor,
        }
    }
}

/// The `Ok` and `Err` `MatchResult`s returned by [`MatchResult::match_result`].
pub(crate) struct ResultMatchers {
    pub(crate) ok: MatchResult,
    pub(crate) err: MatchResult,
}
