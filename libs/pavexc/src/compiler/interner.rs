use std::hash::Hash;

use ahash::{HashMap, HashMapExt};

#[derive(Debug)]
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
    pub fn new() -> Self {
        Self::default()
    }

    pub fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = (la_arena::Idx<T>, &T)> + DoubleEndedIterator {
        self.arena.iter()
    }

    pub fn values(&self) -> impl ExactSizeIterator<Item = &T> + DoubleEndedIterator {
        self.arena.values()
    }
}

impl<T> Interner<T>
where
    T: Hash + Eq + Clone,
{
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

    #[allow(unused)]
    pub fn get(&self, value: &T) -> Option<la_arena::Idx<T>> {
        self.item2id.get(value).copied()
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
