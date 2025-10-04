use indexmap::IndexMap;

/// A routing path.
pub struct RoutePath {
    /// The raw path template provided by the user.
    pub raw: String,
    /// Name <> info mapping for all path parameters.
    pub parameters: IndexMap<String, PathParameterDetails>,
}

pub struct PathParameterDetails {
    /// The index of the opening brace in the raw path.
    pub start: usize,
    /// The index of the closing brace in the raw path.
    pub end: usize,
    /// `true` if the parameter is a catch-all parameter (i.e. `*` is prefixed to its name)
    pub catch_all: bool,
}

impl RoutePath {
    /// Create a new `RoutePath` from a raw path template.
    ///
    /// It extracts path parameters out of a templated path, e.g. `/users/{user_id}/posts/{post_id}`.
    /// or `/users/usr_{user_id}/{*any}`.
    /// Curly braces can also be escaped (e.g. `{{` or `}}`), so we need to handle that.
    pub fn parse(raw: String) -> Self {
        struct CurrentParam {
            start: usize,
            catch_all: bool,
            name: String,
        }

        impl CurrentParam {
            fn reset(&mut self, start: usize) {
                self.start = start;
                self.catch_all = false;
                self.name.clear();
            }
        }

        let mut parameters = IndexMap::new();
        let mut chars = raw.chars().enumerate().peekable();
        let mut inside_braces = false;

        let mut current_param = CurrentParam {
            start: 0,
            catch_all: false,
            name: String::new(),
        };

        while let Some((position, c)) = chars.next() {
            match c {
                '{' => {
                    let next = chars.peek();
                    if let Some((_, '{')) = next {
                        // Skip escaped brace
                        chars.next();
                    } else {
                        inside_braces = true;
                        current_param.reset(position);
                        if let Some((_, '*')) = next {
                            // Skip the catch-all asterisk
                            chars.next();
                            current_param.catch_all = true;
                        }
                    }
                }
                '}' => {
                    if let Some((_, '}')) = chars.peek() {
                        // Skip escaped brace
                        chars.next();
                    } else if inside_braces {
                        inside_braces = false;
                        let info = PathParameterDetails {
                            start: current_param.start,
                            end: position,
                            catch_all: current_param.catch_all,
                        };
                        parameters.insert(current_param.name.clone(), info);
                    }
                }
                _ => {
                    if inside_braces {
                        current_param.name.push(c);
                    }
                }
            }
        }

        Self { raw, parameters }
    }
}
