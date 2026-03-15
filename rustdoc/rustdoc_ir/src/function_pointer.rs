use std::fmt::{self, Debug, Formatter};

use rustdoc_types::Abi;

use crate::Type;

/// A single input parameter for a function pointer type.
#[derive(serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash, Clone)]
pub struct FunctionPointerInput {
    /// The name of the parameter, if any.
    ///
    /// `None` for unnamed parameters (e.g. `fn(usize)`),
    /// `Some` for named parameters (e.g. `fn(a: usize)`).
    pub name: Option<String>,
    /// The type of the parameter.
    pub type_: Type,
}

/// A Rust function pointer type—e.g. `fn(u32) -> u8` or `unsafe extern "C" fn()`.
#[derive(serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash, Clone)]
pub struct FunctionPointer {
    /// The input parameters, including optional names and types.
    pub inputs: Vec<FunctionPointerInput>,
    /// The return type. `None` means the unit type `()`.
    pub output: Option<Box<Type>>,
    /// The ABI of the function pointer (e.g. `Abi::Rust`, `Abi::C { unwind: false }`).
    pub abi: Abi,
    /// Whether this is an `unsafe` function pointer.
    pub is_unsafe: bool,
}

/// Write the `unsafe` and/or `extern "..."` prefix for a function pointer into `f`.
///
/// - If `is_unsafe`, writes `unsafe `.
/// - If `abi` is not `Abi::Rust`, writes `extern "..." `.
pub fn write_fn_pointer_prefix(f: &mut impl fmt::Write, abi: &Abi, is_unsafe: bool) -> fmt::Result {
    if is_unsafe {
        write!(f, "unsafe ")?;
    }
    if let Some(abi_str) = abi_to_str(abi) {
        write!(f, "extern \"{abi_str}\" ")?;
    }
    Ok(())
}

/// Convert an [`Abi`] to the string that appears inside `extern "..."`.
///
/// Returns `None` for `Abi::Rust` (the default, which omits the `extern` keyword).
fn abi_to_str(abi: &Abi) -> Option<&str> {
    match abi {
        Abi::Rust => None,
        Abi::C { unwind: false } => Some("C"),
        Abi::C { unwind: true } => Some("C-unwind"),
        Abi::Cdecl { unwind: false } => Some("cdecl"),
        Abi::Cdecl { unwind: true } => Some("cdecl-unwind"),
        Abi::Stdcall { unwind: false } => Some("stdcall"),
        Abi::Stdcall { unwind: true } => Some("stdcall-unwind"),
        Abi::Fastcall { unwind: false } => Some("fastcall"),
        Abi::Fastcall { unwind: true } => Some("fastcall-unwind"),
        Abi::Aapcs { unwind: false } => Some("aapcs"),
        Abi::Aapcs { unwind: true } => Some("aapcs-unwind"),
        Abi::Win64 { unwind: false } => Some("win64"),
        Abi::Win64 { unwind: true } => Some("win64-unwind"),
        Abi::SysV64 { unwind: false } => Some("sysv64"),
        Abi::SysV64 { unwind: true } => Some("sysv64-unwind"),
        Abi::System { unwind: false } => Some("system"),
        Abi::System { unwind: true } => Some("system-unwind"),
        Abi::Other(s) => Some(s),
    }
}

impl Debug for FunctionPointer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write_fn_pointer_prefix(f, &self.abi, self.is_unsafe)?;
        write!(f, "fn(")?;
        for (i, input) in self.inputs.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            if let Some(name) = &input.name {
                write!(f, "{name}: ")?;
            }
            write!(f, "{:?}", input.type_)?;
        }
        write!(f, ")")?;
        if let Some(output) = &self.output {
            write!(f, " -> {:?}", output)?;
        }
        Ok(())
    }
}
