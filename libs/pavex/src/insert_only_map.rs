use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::hash::Hash;
use std::mem::MaybeUninit;
use std::ops::Deref;

pub struct InsertOnlyMap<const SIZE: usize, Key, Value> {
    storage: [RefCell<MaybeUninit<Value>>; SIZE],
    id2element: RefCell<HashMap<Key, usize>>,
    next_index: RefCell<usize>,
}

impl<const SIZE: usize, Key, Value> InsertOnlyMap<SIZE, Key, Value> {
    pub fn new() -> Self {
        Self {
            storage: std::array::from_fn(|_| RefCell::new(MaybeUninit::uninit())),
            id2element: RefCell::new(HashMap::new()),
            next_index: RefCell::new(0),
        }
    }

    pub fn insert(&self, key: Key, value: Value) -> Result<(), InsertError>
    where
        Key: Hash + Eq,
    {
        if self.id2element.borrow().get(&key).is_some() {
            return Err(InsertError::KeyIsAlreadyInUse(KeyIsAlreadyInUse));
        }
        let new_element_index = self.next_index.borrow().to_owned();
        *self.next_index.borrow_mut() += 1;
        *self.storage[new_element_index].borrow_mut() = MaybeUninit::new(value);
        self.id2element.borrow_mut().insert(key, new_element_index);
        Ok(())
    }

    pub fn get(&self, key: &Key) -> Option<ValueRef<Value>>
    where
        Key: Hash + Eq,
    {
        let element_id = match self.id2element.borrow().get(key) {
            None => return None,
            Some(&element_id) => element_id,
        };
        Some(ValueRef(self.storage[element_id].borrow()))
    }
}

impl<const SIZE: usize, Key, Value> Drop for InsertOnlyMap<SIZE, Key, Value> {
    fn drop(&mut self) {
        // `MaybeUninit` does nothing on drop.
        // We need to manually drop every element we allocated.
        for elem in &mut self.storage[0..*self.next_index.borrow()] {
            let mut ref_mut = (*elem).borrow_mut();
            unsafe {
                std::ptr::drop_in_place(ref_mut.as_mut_ptr());
            }
        }
    }
}

pub struct ValueRef<'a, Value>(Ref<'a, MaybeUninit<Value>>);

impl<'a, Value> Deref for ValueRef<'a, Value> {
    type Target = Value;

    fn deref(&self) -> &Self::Target {
        // This is safe since `InsertOnlyMap::get` only returns `Some(ValueRef)`
        // for indexes that point to memory locations in the storage array that
        // have been initialised.
        unsafe { self.0.assume_init_ref() }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum InsertError {
    #[error(transparent)]
    KeyIsAlreadyInUse(#[from] KeyIsAlreadyInUse),
    #[error(transparent)]
    MapIsFull(#[from] MapIsFull),
}

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
#[error("There is already a value stored in the map for the key you provided.")]
struct KeyIsAlreadyInUse;

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
#[error("The map is full.")]
struct MapIsFull;
