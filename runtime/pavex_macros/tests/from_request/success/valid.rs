use pavex_macros::FromRequest;

#[derive(FromRequest)]
pub struct OnePathParam {
    #[path_param]
    pp_a: u64,
}

#[derive(FromRequest)]
pub struct OneQueryParam {
    #[query_param]
    qa: u64,
}

#[derive(FromRequest)]
pub struct JustBody {
    #[body(format = "json")]
    body: String,
}

#[derive(FromRequest)]
pub struct PathAndQuery {
    #[path_param]
    pp_a: u64,
    #[query_param]
    qa: u64,
}

#[derive(FromRequest)]
pub struct PathAndBody {
    #[path_param]
    pp_a: u64,
    #[body(format = "json")]
    body: String,
}

#[derive(FromRequest)]
pub struct QueryAndBody {
    #[query_param]
    qa: u64,
    #[body(format = "json")]
    body: String,
}

#[derive(FromRequest)]
pub struct PathQueryAndBody {
    #[path_param]
    pp_a: u64,
    #[query_param]
    qa: u64,
    #[body(format = "json")]
    body: String,
}

#[derive(FromRequest)]
pub struct MultiplePathParams {
    #[path_param]
    pp_a: u64,
    #[path_param(name = "bb")]
    pp_b: u64,
}

#[derive(FromRequest)]
pub struct MultipleQueryParams {
    #[query_param]
    qa: u64,
    #[query_param(name = "bb")]
    qb: u64,
}

#[derive(FromRequest)]
pub struct AllTogether {
    #[path_param]
    pp_a: u64,
    #[path_param(name = "bb")]
    pp_b: u64,
    #[path_param]
    pp_c: u64,

    #[path_params]
    pp_multiple: Vec<u64>,

    #[query_param]
    qa: u64,
    #[query_param(name = "bb")]
    qb: u64,

    #[query_params]
    qps: Vec<u64>,

    #[body(format = "json")]
    body: String,
}

fn main() {}
