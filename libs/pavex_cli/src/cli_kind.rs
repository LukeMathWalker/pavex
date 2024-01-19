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

    pub fn binary_filename(self) -> &'static str {
        cfg_if::cfg_if! {
            if #[cfg(windows)] {
                match self {
                    CliKind::Pavex => "pavex.exe",
                    CliKind::Pavexc => "pavexc.exe",
                }
            } else {
                match self {
                    CliKind::Pavex => "pavex",
                    CliKind::Pavexc => "pavexc",
                }
            }
        }
    }
}
