use ahash::{HashMap, HashMapExt};

/// To make the comparison easier, we assign a monotonically increasing unique id to all
/// unassigned generic parameters.
/// If the ids match, we know that the two sequences of unassigned generic parameters are equivalent.
pub(crate) struct UnassignedIdGenerator<'a> {
    next_id: usize,
    known_ids: HashMap<&'a str, usize>,
}

impl<'a> UnassignedIdGenerator<'a> {
    pub(crate) fn new() -> Self {
        Self {
            next_id: 0,
            known_ids: HashMap::new(),
        }
    }

    pub(crate) fn id<'b>(&'b mut self, name: &'a str) -> usize
    where
        'a: 'b,
    {
        if let Some(id) = self.known_ids.get(&name) {
            *id
        } else {
            let id = self.next_id;
            self.next_id += 1;
            self.known_ids.insert(name, id);
            id
        }
    }

    /// Iterate over the known ids, sorted by their assigned ID (i.e. insertion order).
    ///
    /// This is important because `HashMap` iteration order is arbitrary,
    /// and callers rely on pairing entries by position across two generators.
    pub(crate) fn into_sorted_iter(self) -> impl Iterator<Item = (&'a str, usize)> {
        let mut entries: Vec<_> = self.known_ids.into_iter().collect();
        entries.sort_by_key(|(_, id)| *id);
        entries.into_iter()
    }
}
