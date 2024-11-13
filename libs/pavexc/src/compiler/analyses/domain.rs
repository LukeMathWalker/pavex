use indexmap::IndexSet;

/// A routing constraint on the domain specified by the caller in the `Host` header.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct DomainGuard {
    domain: String,
}

impl std::fmt::Display for DomainGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.domain)
    }
}

impl DomainGuard {
    /// Validate the user-provided domain constraint and create a new `DomainGuard`,
    /// or return an error if the domain constraint is invalid.
    pub(crate) fn new(domain: String) -> Result<Self, InvalidDomainConstraint> {
        validate(&domain)?;
        // Normalize by skipping the last trailing dot, if any
        let domain = domain.trim_end_matches('.').to_string();
        Ok(Self { domain })
    }

    /// Return a pattern that can be used to match the domain constraint against a request's host
    /// header using the `matchit` crate.
    ///
    /// We reverse the domain (i.e. `{sub}.example.com` becomes `moc.elpmaxe.{sub}`) to maximize
    /// the shared prefix between different patterns as well as allowing the use of wildcard
    /// parameters to capture arbitrarily nested subdomains.
    /// We also replace `.` with `/` since `matchit` is designed for path routing and uses `/` as
    /// the path separator.
    pub(crate) fn matchit_pattern(&self) -> String {
        let mut reversed = String::with_capacity(self.domain.len());
        let mut chars = self.domain.chars().rev().peekable();

        loop {
            let Some(c) = chars.next() else {
                break;
            };
            if c == '.' {
                reversed.push('/');
            } else if c == '}' {
                // We don't reverse the parameter name, since it's the Rust identifier
                // the user wants to refer to in their handlers/components.
                let parameter_name = chars.by_ref().take_while(|c| *c != '{').collect::<String>();
                reversed.push('{');
                reversed.extend(parameter_name.chars().rev());
                reversed.push(c);
            } else {
                reversed.push(c);
            }
        }
        reversed
    }
}

/// Return a meaningful error message if the domain constraint is invalid.
fn validate(input: &str) -> Result<(), InvalidDomainConstraint> {
    struct ParsedParameter {
        start_at: usize,
        end_at: usize,
        is_catch_all: bool,
    }

    impl ParsedParameter {
        /// Extract the raw parameter from the input string.
        /// E.g. `{*param_name}`.
        fn raw(&self, label: &str) -> String {
            label
                .chars()
                .skip(self.start_at)
                .take(self.end_at - self.start_at + 1)
                .collect()
        }
    }

    struct CurrentParameter {
        name: String,
        start_at: usize,
        is_catch_all: bool,
    }

    let has_trailing_dot = input.ends_with('.');

    if input.is_empty() {
        return Err(InvalidDomainConstraint::Empty);
    }

    // Each segment of a domain is called a "DNS label". Labels are separated by dots.
    let labels = {
        let mut i = input.split('.');
        if has_trailing_dot {
            // Skip the last empty label to avoid returning an incorrect error message
            i.next_back();
        }
        i
    };
    let mut total_length = 0;
    let mut is_template = false;

    for (label_index, label) in labels.enumerate() {
        total_length += 1; // For the dot separator

        if label.is_empty() {
            return Err(InvalidDomainConstraint::EmptyDnsLabel {
                original: input.to_string(),
            });
        }

        let mut label_length = 0;
        let mut parsed_parameters: Vec<ParsedParameter> = Vec::new();
        let mut parameter: Option<CurrentParameter> = None;
        let mut invalid_label_chars = IndexSet::new();
        let mut is_label_template = false;

        let mut chars = label.chars().enumerate().peekable();
        loop {
            let Some((i, char)) = chars.next() else {
                // We're at the end of the label.

                // Were we in the middle of parsing a domain parameter?
                if let Some(parameter) = parameter {
                    return Err(InvalidDomainConstraint::UnclosedParameter {
                        original: input.to_string(),
                        dns_label: label.into(),
                        partial_parameter: label.chars().skip(parameter.start_at).collect(),
                    });
                }
                // Do we have any invalid characters?
                if !invalid_label_chars.is_empty() {
                    return Err(InvalidDomainConstraint::InvalidDnsLabel {
                        original: input.into(),
                        invalid_label: label.into(),
                        violations: DnsLabelViolations::InvalidChars(invalid_label_chars),
                    });
                }

                if parsed_parameters.len() > 1 {
                    return Err(InvalidDomainConstraint::InvalidDnsLabel {
                        original: input.to_string(),
                        invalid_label: label.to_string(),
                        violations: DnsLabelViolations::TooManyParameters {
                            parameters: parsed_parameters.iter().map(|p| p.raw(label)).collect(),
                        },
                    });
                }

                if let Some(p) = parsed_parameters.first() {
                    if p.start_at != 0 {
                        return Err(InvalidDomainConstraint::InvalidDnsLabel {
                            original: input.to_string(),
                            invalid_label: label.to_string(),
                            violations: DnsLabelViolations::ParameterNotAtStart {
                                parameter: p.raw(label),
                                prefix: label.chars().take(p.start_at).collect(),
                            },
                        });
                    }

                    if p.is_catch_all && label_index != 0 {
                        return Err(InvalidDomainConstraint::CatchAllNotAtStart {
                            original: input.to_string(),
                            parameter: p.raw(label),
                        });
                    }
                }

                total_length += label_length;
                break;
            };

            if char == '}' {
                if let Some(p) = std::mem::take(&mut parameter) {
                    // We only add 1 since that's minimum length of whatever matches the parameter
                    label_length += 1;

                    is_label_template = true;

                    if p.name.is_empty() {
                        return Err(InvalidDomainConstraint::EmptyParameterName {
                            original: input.to_string(),
                            unnamed_parameter: if p.is_catch_all { "{*}" } else { "{}" },
                        });
                    }

                    // Check if it's a valid Rust identifier using syn
                    if syn::parse_str::<syn::Ident>(&p.name).is_err() {
                        return Err(InvalidDomainConstraint::InvalidParameterName {
                            original: input.to_string(),
                            parameter_name: p.name,
                        });
                    }

                    let parsed_parameter = ParsedParameter {
                        start_at: p.start_at,
                        end_at: i,
                        is_catch_all: p.is_catch_all,
                    };
                    parsed_parameters.push(parsed_parameter);
                } else {
                    label_length += 1;
                    invalid_label_chars.insert(char);
                }
            } else if char == '{' {
                if let Some(parameter) = &mut parameter {
                    parameter.name.push(char);
                } else {
                    let is_catch_all = if let Some((_, '*')) = chars.peek() {
                        // Consume the '*' character
                        chars.next();
                        true
                    } else {
                        false
                    };
                    parameter = Some(CurrentParameter {
                        name: String::new(),
                        start_at: i,
                        is_catch_all,
                    });
                }
            } else {
                if let Some(parameter) = &mut parameter {
                    parameter.name.push(char);
                } else {
                    label_length += 1;
                    if !(char.is_ascii_alphanumeric() || char == '-') {
                        invalid_label_chars.insert(char);
                    }
                }
            }
        }
        let first = label.chars().next().unwrap();
        if first != '{' && !first.is_ascii_alphanumeric() {
            return Err(InvalidDomainConstraint::InvalidDnsLabel {
                original: input.into(),
                invalid_label: label.into(),
                violations: DnsLabelViolations::InvalidStart,
            });
        }

        let last = label.chars().last().unwrap();
        if last != '}' && !last.is_ascii_alphanumeric() {
            return Err(InvalidDomainConstraint::InvalidDnsLabel {
                original: input.into(),
                invalid_label: label.into(),
                violations: DnsLabelViolations::InvalidEnd,
            });
        }

        if label_length > 63 {
            return Err(InvalidDomainConstraint::InvalidDnsLabel {
                original: input.to_string(),
                invalid_label: label.into(),
                violations: DnsLabelViolations::TooLong {
                    length: label_length,
                    templated: is_label_template,
                },
            });
        }

        is_template |= is_label_template;
    }

    total_length -= 1; // We overcounted dot separators

    if total_length > 253 {
        return Err(InvalidDomainConstraint::TooLong {
            original: input.to_string(),
            length: total_length,
            templated: is_template,
        });
    }

    Ok(())
}

#[derive(Debug)]
pub(crate) enum InvalidDomainConstraint {
    Empty,
    TooLong {
        original: String,
        length: usize,
        templated: bool,
    },
    EmptyDnsLabel {
        original: String,
    },
    CatchAllNotAtStart {
        original: String,
        parameter: String,
    },
    InvalidDnsLabel {
        original: String,
        invalid_label: String,
        violations: DnsLabelViolations,
    },
    InvalidParameterName {
        original: String,
        parameter_name: String,
    },
    EmptyParameterName {
        original: String,
        unnamed_parameter: &'static str,
    },
    UnclosedParameter {
        original: String,
        dns_label: String,
        partial_parameter: String,
    },
}

#[derive(Debug)]
pub(crate) enum DnsLabelViolations {
    InvalidStart,
    InvalidEnd,
    InvalidChars(IndexSet<char>),
    TooLong { length: usize, templated: bool },
    ParameterNotAtStart { parameter: String, prefix: String },
    TooManyParameters { parameters: Vec<String> },
}

impl std::error::Error for InvalidDomainConstraint {}

impl std::fmt::Display for InvalidDomainConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InvalidDomainConstraint::CatchAllNotAtStart {
                original,
                parameter,
            } => {
                write!(
                    f,
                    "Catch-all parameters must appear at the very beginning of the domain.\n\
                    That's not the case for `{}`: there is a catch-all parameter, `{}`, but it doesn't appear first.",
                    original, parameter
                )
            }
            InvalidDomainConstraint::Empty => {
                write!(f, "Domain constraints can't be empty.")
            }
            InvalidDomainConstraint::TooLong {
                original,
                length,
                templated,
            } => {
                if *templated {
                    write!(
                        f,
                        "`{}` is too long to be a valid domain constraint. The maximum allowed length is 253 characters, but this domain constraint would only match domains that are at least {} characters long.",
                        original, length
                    )
                } else {
                    write!(
                        f,
                        "`{}` is too long to be a valid domain. The maximum allowed length is 253 characters, but this domain is {} characters long.",
                        original, length
                    )
                }
            }
            InvalidDomainConstraint::EmptyDnsLabel { original } => {
                write!(f, "`{original}` is not a valid domain. It contains an empty DNS label: two consecutive dots (`..`), with nothing in between.")
            }
            InvalidDomainConstraint::InvalidDnsLabel {
                original,
                invalid_label,
                violations,
            } => {
                write!(f, "`{original}` is not a valid domain. It contains an invalid DNS label, `{invalid_label}`. ")?;
                match violations {
                    DnsLabelViolations::InvalidStart => {
                        let first = invalid_label.chars().next().unwrap();
                        write!(f, "DNS labels must start with an alphanumeric ASCII character, but `{invalid_label}` starts with `{first}`.")
                    }
                    DnsLabelViolations::InvalidEnd => {
                        let last = invalid_label.chars().last().unwrap();
                        write!(f, "DNS labels must end with an alphanumeric ASCII character, but `{invalid_label}` ends with `{last}`.")
                    }
                    DnsLabelViolations::InvalidChars(chars) => {
                        write!(f, "DNS labels must only contain alphanumeric ASCII characters and hyphens (`-`), but `{invalid_label}` contains the following invalid characters: ")?;
                        for (i, c) in chars.iter().enumerate() {
                            if i > 0 {
                                write!(f, ", ")?;
                            }
                            write!(f, "`{}`", c)?;
                        }
                        Ok(())
                    }
                    DnsLabelViolations::TooLong { length, templated } => {
                        if *templated {
                            write!(f, "DNS labels must be at most 63 characters long, but `{invalid_label}` would only match labels that are at least {} characters long.", length)
                        } else {
                            write!(f, "DNS labels must be at most 63 characters long, but `{invalid_label}` is {} characters long.", length)
                        }
                    }
                    DnsLabelViolations::ParameterNotAtStart { parameter, prefix } => {
                        write!(f, "Domain parameters must appear at the beginning of the DNS label they belong to. That's not the case here: `{parameter}` is preceded by `{prefix}`.")
                    }
                    DnsLabelViolations::TooManyParameters { parameters } => {
                        let n = parameters.len();
                        write!(f, "DNS labels can contain at most one domain parameter. `{invalid_label}`, instead, contains {n} parameters: ")?;
                        for (i, p) in parameters.iter().enumerate() {
                            if n > 1 && i == n - 1 {
                                write!(f, " and ")?;
                            } else if i > 0 {
                                write!(f, ", ")?;
                            }
                            write!(f, "`{}`", p)?;
                        }
                        write!(f, ".")
                    }
                }
            }
            InvalidDomainConstraint::InvalidParameterName {
                original,
                parameter_name,
            } => {
                write!(f, "`{parameter_name}`, one of the domain parameters in `{original}`, is not a valid Rust identifier.")
            }
            InvalidDomainConstraint::EmptyParameterName {
                original,
                unnamed_parameter,
            } => {
                write!(f, "All domain parameters must be named. `{original}` can't be accepted since it contains an unnamed parameter, `{unnamed_parameter}`.")
            }
            InvalidDomainConstraint::UnclosedParameter {
                original,
                dns_label,
                partial_parameter,
            } => {
                write!(f, "`{original}` is not a valid domain. It contains an unclosed domain parameter in one of its DNS labels, `{dns_label}`. \
                    Domain parameters must be enclosed in curly braces (`{{` and `}}`), but `{partial_parameter}` is missing a closing brace (`}}`).")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::validate;
    use super::DomainGuard;
    use insta::assert_snapshot;

    macro_rules! is_valid {
        ($guard:expr, $pattern:literal) => {
            let d = DomainGuard::new($guard.into()).unwrap();
            let p = d.matchit_pattern();
            assert_eq!(p, $pattern);
            let mut router = matchit::Router::new();
            // We don't accept anything that `matchit` would later reject.
            router
                .insert(p, ())
                .expect(concat!($pattern, " is not valid for `matchit`"));
        };
    }

    #[test]
    fn test_valid_domains_with_params() {
        is_valid!("sub.{placeholder}.com", "moc/{placeholder}/bus");
        is_valid!("{*param}.example.com", "moc/elpmaxe/{*param}"); // Catch-all parameter
        is_valid!("{param1}.sub.{param2}.com", "moc/{param2}/bus/{param1}");
        is_valid!(
            "{valid_identifier}.domain.com.",
            "moc/niamod/{valid_identifier}"
        ); // Absolute form with trailing dot
        is_valid!(
            "{valid_identifier}some.domain.com.",
            "moc/niamod/emos{valid_identifier}"
        ); // Absolute form with placeholder embedded in a larger label
        is_valid!("example.com", "moc/elpmaxe");
        is_valid!("subdomain.example.com", "moc/elpmaxe/niamodbus");
        is_valid!("a.com", "moc/a");
        is_valid!("valid-domain.com", "moc/niamod-dilav");
        is_valid!("sub.valid-domain.com", "moc/niamod-dilav/bus");
    }

    #[test]
    fn test_edge_cases() {
        // Single label domain (allowed in FQDN form)
        assert!(validate("com").is_ok());
        assert!(validate("com.").is_ok()); // Absolute form

        // Domain with maximum label length
        let max_label = "a".repeat(63);
        assert!(validate(&format!("{max_label}.example.com")).is_ok());

        // Domain with label exceeding maximum length
        let long_label = "a".repeat(64);
        assert_snapshot!(validate(&format!("{long_label}.example.com")).unwrap_err(), @"`aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.example.com` is not a valid domain. It contains an invalid DNS label, `aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa`. DNS labels must be at most 63 characters long, but `aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa` is 64 characters long.");

        // Domain with exactly 253 characters
        let label = "a".repeat(35);
        let mut domain = String::new();
        for _ in 0..7 {
            domain.push_str(&label);
            domain.push('.');
        }
        domain.push('c');
        assert_eq!(domain.len(), 253);
        assert!(validate(&domain).is_ok());

        // Domain with exactly 253 characters, when ignoring the trailing dot in the absolute form
        let domain = format!("{domain}.");
        assert!(validate(&domain).is_ok());
    }

    #[test]
    fn test_invalid_domains() {
        assert_snapshot!(
            validate("").unwrap_err(),
            @"Domain constraints can't be empty."
        );

        // Invalid characters in domain
        assert_snapshot!(
            validate("example!.com").unwrap_err(),
            @"`example!.com` is not a valid domain. It contains an invalid DNS label, `example!`. DNS labels must only contain alphanumeric ASCII characters and hyphens (`-`), but `example!` contains the following invalid characters: `!`"
        );

        assert_snapshot!(
            validate("exa mple.com").unwrap_err(),
            @"`exa mple.com` is not a valid domain. It contains an invalid DNS label, `exa mple`. DNS labels must only contain alphanumeric ASCII characters and hyphens (`-`), but `exa mple` contains the following invalid characters: ` `"
        );

        // Empty DNS labels
        assert_snapshot!(
            validate("example..com").unwrap_err(),
            @"`example..com` is not a valid domain. It contains an empty DNS label: two consecutive dots (`..`), with nothing in between."
        );

        assert_snapshot!(
            validate(".example.com").unwrap_err(),
            @"`.example.com` is not a valid domain. It contains an empty DNS label: two consecutive dots (`..`), with nothing in between."
        );

        // Invalid starting or ending characters
        assert_snapshot!(
            validate("-example.com").unwrap_err(),
            @"`-example.com` is not a valid domain. It contains an invalid DNS label, `-example`. DNS labels must start with an alphanumeric ASCII character, but `-example` starts with `-`."
        );

        assert_snapshot!(
            validate("example-.com").unwrap_err(),
            @"`example-.com` is not a valid domain. It contains an invalid DNS label, `example-`. DNS labels must end with an alphanumeric ASCII character, but `example-` ends with `-`."
        );
    }

    #[test]
    fn test_invalid_parameters() {
        // Unclosed parameter
        assert_snapshot!(
            validate("{unclosed.example.com").unwrap_err(),
            @"`{unclosed.example.com` is not a valid domain. It contains an unclosed domain parameter in one of its DNS labels, `{unclosed`. Domain parameters must be enclosed in curly braces (`{` and `}`), but `{unclosed` is missing a closing brace (`}`)."
        );

        // Empty parameter name
        assert_snapshot!(
            validate("{}.example.com").unwrap_err(),
            @"All domain parameters must be named. `{}.example.com` can't be accepted since it contains an unnamed parameter, `{}`."
        );

        assert_snapshot!(
            validate("{*}.example.com").unwrap_err(),
            @"All domain parameters must be named. `{*}.example.com` can't be accepted since it contains an unnamed parameter, `{*}`."
        );

        // Invalid parameter name
        assert_snapshot!(
            validate("{invalid-param}.example.com").unwrap_err(),
            @"`invalid-param`, one of the domain parameters in `{invalid-param}.example.com`, is not a valid Rust identifier."
        );

        assert_snapshot!(
            validate("{9invalid}.example.com").unwrap_err(),
            @"`9invalid`, one of the domain parameters in `{9invalid}.example.com`, is not a valid Rust identifier."
        );

        // Text preceding a parameter inside the same label
        assert_snapshot!(
            validate("some{valid_identifier}.domain.com.").unwrap_err(),
            @"`some{valid_identifier}.domain.com.` is not a valid domain. It contains an invalid DNS label, `some{valid_identifier}`. Domain parameters must appear at the beginning of the DNS label they belong to. That's not the case here: `{valid_identifier}` is preceded by `some`."
        );

        // A non-leading catch-all parameter
        assert_snapshot!(
            validate("sub.{*all}.domain.com.").unwrap_err(),
            @r###"
        Catch-all parameters must appear at the very beginning of the domain.
        That's not the case for `sub.{*all}.domain.com.`: there is a catch-all parameter, `{*all}`, but it doesn't appear first.
        "###
        );

        // Multiple parameters in the same label
        assert_snapshot!(
            validate("sub.{param1}{param2}.domain.com.").unwrap_err(),
            @"`sub.{param1}{param2}.domain.com.` is not a valid domain. It contains an invalid DNS label, `{param1}{param2}`. DNS labels can contain at most one domain parameter. `{param1}{param2}`, instead, contains 2 parameters: `{param1}` and `{param2}`."
        );
    }

    #[test]
    fn test_invalid_length() {
        // Domain length exceeds 253 characters
        let label = "a".repeat(36);
        let mut domain = String::new();
        for _ in 0..7 {
            domain.push_str(&label);
            domain.push('.');
        }

        assert_snapshot!(validate(&domain).unwrap_err(), @"`aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.` is too long to be a valid domain. The maximum allowed length is 253 characters, but this domain is 258 characters long.");
    }
}
