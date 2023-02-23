use std::fmt;
use std::fmt::Display;

#[derive(Debug, Clone, Copy)]
/// A wrapper around a `usize` that formats as an ordinal number.
/// The numbers are 0-based, i.e. `0` is formatted as `first`, `1` as `second`, etc.
pub struct ZeroBasedOrdinal(pub usize);

impl From<usize> for ZeroBasedOrdinal {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl Display for ZeroBasedOrdinal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 + 1 {
            1 => write!(f, "first"),
            2 => write!(f, "second"),
            3 => write!(f, "third"),
            n => {
                let last_two_digits = n % 100;
                let last_digit = last_two_digits % 10;
                if last_digit == 1 && last_two_digits != 11 {
                    write!(f, "{}st", n)
                } else if last_digit == 2 && last_two_digits != 12 {
                    write!(f, "{}nd", n)
                } else if last_digit == 3 && last_two_digits != 13 {
                    write!(f, "{}rd", n)
                } else {
                    write!(f, "{}th", n)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::diagnostic::ordinals::ZeroBasedOrdinal;

    #[test]
    fn test_0_based() {
        assert_eq!(format!("{}", ZeroBasedOrdinal(0)), "first");
        assert_eq!(format!("{}", ZeroBasedOrdinal(1)), "second");
        assert_eq!(format!("{}", ZeroBasedOrdinal(2)), "third");
        assert_eq!(format!("{}", ZeroBasedOrdinal(3)), "4th");
        assert_eq!(format!("{}", ZeroBasedOrdinal(10)), "11th");
        assert_eq!(format!("{}", ZeroBasedOrdinal(11)), "12th");
        assert_eq!(format!("{}", ZeroBasedOrdinal(12)), "13th");
        assert_eq!(format!("{}", ZeroBasedOrdinal(30)), "31st");
        assert_eq!(format!("{}", ZeroBasedOrdinal(41)), "42nd");
        assert_eq!(format!("{}", ZeroBasedOrdinal(52)), "53rd");
        assert_eq!(format!("{}", ZeroBasedOrdinal(53)), "54th");
    }
}
