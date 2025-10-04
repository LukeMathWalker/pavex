use semver::{Comparator, Op, Version, VersionReq};

/// Given an exact semver version, returns a matcher that matches any semver-compatible
/// version.  
/// In the end, `cargo` is in charge of resolving the actual version, so we don't need to
/// be too strict here.
///
/// This is a good defense against crates setting a stale `#![doc(html_root_url = "...")]`
/// (e.g. https://github.com/hyperium/http/pull/688).
pub(super) struct VersionMatcher {
    req: VersionReq,
}

impl VersionMatcher {
    pub(super) fn new(version: &Version) -> VersionMatcher {
        let mut comparator = Comparator {
            op: Op::Caret,
            major: version.major,
            minor: None,
            patch: None,
            pre: Default::default(),
        };
        if version.pre.is_empty() {
            if version.major == 0 {
                comparator.minor = Some(version.minor);
                if version.minor == 0 {
                    comparator.patch = Some(version.patch);
                }
            }
        } else {
            // We don't play loose with pre-releases.
            comparator.minor = Some(version.minor);
            comparator.patch = Some(version.patch);
            comparator.pre = version.pre.clone();
        }
        let req = VersionReq {
            comparators: vec![comparator],
        };

        VersionMatcher { req }
    }

    pub(super) fn matches(&self, version: &Version) -> bool {
        self.req.matches(version)
    }
}
