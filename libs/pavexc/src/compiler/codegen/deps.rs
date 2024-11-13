use ahash::HashMap;
use bimap::BiHashMap;
use guppy::PackageId;
use quote::format_ident;
use syn::Ident;

/// The source of truth for the names used to import third-party dependencies from the generated code.
pub struct ServerSdkDeps {
    pavex: (PackageId, Ident),
    http: (PackageId, Ident),
    hyper: (PackageId, Ident),
    thiserror: (PackageId, Ident),
    matchit: (PackageId, Ident),
}

impl ServerSdkDeps {
    pub fn new(
        codegen_deps: &HashMap<String, PackageId>,
        package_id2name: &BiHashMap<PackageId, String>,
    ) -> Self {
        let import_name = |pkg_id: &PackageId| {
            let import_name = package_id2name.get_by_left(pkg_id).unwrap();
            format_ident!("{}", import_name)
        };

        let pavex_pkg_id = codegen_deps["pavex"].clone();
        let pavex_import_name = import_name(&pavex_pkg_id);

        let http_pkg_id = codegen_deps["http"].clone();
        let http_import_name = import_name(&http_pkg_id);

        let hyper_pkg_id = codegen_deps["hyper"].clone();
        let hyper_import_name = import_name(&hyper_pkg_id);

        let thiserror_pkg_id = codegen_deps["thiserror"].clone();
        let thiserror_import_name = import_name(&thiserror_pkg_id);

        let matchit_pkg_id = codegen_deps["matchit"].clone();
        let matchit_import_name = import_name(&matchit_pkg_id);

        Self {
            pavex: (pavex_pkg_id, pavex_import_name),
            http: (http_pkg_id, http_import_name),
            hyper: (hyper_pkg_id, hyper_import_name),
            thiserror: (thiserror_pkg_id, thiserror_import_name),
            matchit: (matchit_pkg_id, matchit_import_name),
        }
    }

    pub fn pavex_ident(&self) -> &Ident {
        &self.pavex.1
    }

    pub fn http_ident(&self) -> &Ident {
        &self.http.1
    }

    pub fn hyper_ident(&self) -> &Ident {
        &self.hyper.1
    }

    pub fn thiserror_ident(&self) -> &Ident {
        &self.thiserror.1
    }

    pub fn matchit_ident(&self) -> &Ident {
        &self.matchit.1
    }
}
