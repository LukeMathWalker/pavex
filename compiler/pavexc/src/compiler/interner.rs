use std::hash::Hash;

use ahash::{HashMap, HashMapExt};

#[derive(Debug)]
/// A simple interner to associate unique (cheap) identifiers to every distinct
/// instance of a type `T`.
///
/// Comparing identifiers is cheaper than comparing the raw `T` values directly,
/// and it allows us to avoid cloning `T` values everywhere whenever we need
/// to reference them.
///
/// # Implementation notes
///
/// There's almost surely a more efficient way to implement this interner,
/// but we left optimizations for later.
pub(crate) struct Interner<T> {
    arena: la_arena::Arena<T>,
    item2id: HashMap<T, la_arena::Idx<T>>,
}

impl<T> Default for Interner<T> {
    fn default() -> Self {
        Self {
            arena: la_arena::Arena::new(),
            item2id: HashMap::new(),
        }
    }
}

impl<T> Interner<T> {
    /// Create a new interner.
    pub fn new() -> Self {
        Self::default()
    }

    /// Iterate over all interned items and their ids.
    pub fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = (la_arena::Idx<T>, &T)> + DoubleEndedIterator {
        self.arena.iter()
    }
}

impl<T> Interner<T>
where
    T: Hash + Eq + Clone,
{
    /// Intern a value, returning its id.
    ///
    /// If the value is already interned, return its id without storing an
    /// additional copy.
    pub fn get_or_intern(&mut self, value: T) -> la_arena::Idx<T> {
        match self.item2id.get(&value) {
            Some(id) => *id,
            _ => {
                let id = self.arena.alloc(value.clone());
                self.item2id.insert(value, id);
                id
            }
        }
    }
}

impl<T: Hash + Eq> std::ops::Index<la_arena::Idx<T>> for Interner<T> {
    type Output = T;

    fn index(&self, index: la_arena::Idx<T>) -> &Self::Output {
        &self.arena[index]
    }
}

impl<T: Hash + Eq> std::ops::Index<&T> for Interner<T> {
    type Output = la_arena::Idx<T>;

    fn index(&self, index: &T) -> &Self::Output {
        &self.item2id[index]
    }
}
