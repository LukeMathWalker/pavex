pub use compute::{get_crate_data, CannotGetCrateData};
pub use package_id_spec::PackageIdSpecification;
pub use queries::{Crate, CrateCollection, UnknownTypePath};

mod compute;
mod package_id_spec;
mod queries;

pub const STD_PACKAGE_ID: &str = "std";
pub const TOOLCHAIN_CRATES: [&str; 3] = ["std", "core", "alloc"];
