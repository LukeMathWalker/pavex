use std::fmt::{Debug, Display, Formatter};

/// A Rust scalar primitive type (e.g. `u32`, `bool`, `str`).
#[derive(serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash, Clone)]
pub enum ScalarPrimitive {
    Usize,
    U8,
    U16,
    U32,
    U64,
    U128,
    Isize,
    I8,
    I16,
    I32,
    I64,
    I128,
    F32,
    F64,
    Bool,
    Char,
    Str,
}

impl ScalarPrimitive {
    /// Returns the primitive type name as a string slice (e.g. `"u32"`, `"bool"`).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Usize => "usize",
            Self::U8 => "u8",
            Self::U16 => "u16",
            Self::U32 => "u32",
            Self::U64 => "u64",
            Self::U128 => "u128",
            Self::Isize => "isize",
            Self::I8 => "i8",
            Self::I16 => "i16",
            Self::I32 => "i32",
            Self::I64 => "i64",
            Self::I128 => "i128",
            Self::F32 => "f32",
            Self::F64 => "f64",
            Self::Bool => "bool",
            Self::Char => "char",
            Self::Str => "str",
        }
    }
}

impl Debug for ScalarPrimitive {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Display for ScalarPrimitive {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Error returned when trying to convert an unrecognized string into a [`ScalarPrimitive`].
#[derive(thiserror::Error, Debug)]
#[error("Unknown primitive type, `{name}`")]
pub struct UnknownPrimitive {
    /// The unrecognized primitive type name.
    pub name: String,
}

impl TryFrom<&str> for ScalarPrimitive {
    type Error = UnknownPrimitive;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let v = match value {
            "usize" => Self::Usize,
            "u8" => Self::U8,
            "u16" => Self::U16,
            "u32" => Self::U32,
            "u64" => Self::U64,
            "u128" => Self::U128,
            "isize" => Self::Isize,
            "i8" => Self::I8,
            "i16" => Self::I16,
            "i32" => Self::I32,
            "i64" => Self::I64,
            "i128" => Self::I128,
            "f32" => Self::F32,
            "f64" => Self::F64,
            "bool" => Self::Bool,
            "char" => Self::Char,
            "str" => Self::Str,
            _ => {
                return Err(UnknownPrimitive {
                    name: value.to_string(),
                });
            }
        };
        Ok(v)
    }
}
