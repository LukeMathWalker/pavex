#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliKind {
    Pavex,
    Pavexc,
}

impl CliKind {
    pub fn binary_target_name(self) -> &'static str {
        match self {
            CliKind::Pavex => "pavex",
            CliKind::Pavexc => "pavexc",
        }
    }

    pub fn package_name(self) -> &'static str {
        match self {
            CliKind::Pavex => "pavex_cli",
            CliKind::Pavexc => "pavexc_cli",
        }
    }

    pub fn binary_filename(self) -> String {
        let name = match self {
            CliKind::Pavex => "pavex",
            CliKind::Pavexc => "pavexc",
        };
        format!("{}{}", name, std::env::consts::EXE_SUFFIX)
    }
}
