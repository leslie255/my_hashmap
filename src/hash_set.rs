use std::{
    fmt::{self, Debug},
    hash::Hash,
};

use crate::hash_map::{self, HashMap};

#[derive(Clone)]
pub struct HashSet<T> {
    map: HashMap<T, ()>,
}

impl<T: Debug> Debug for HashSet<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

impl<T> HashSet<T> {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            map: HashMap::with_capacity(capacity),
        }
    }

    pub fn iter(&self) -> Iter<T> {
        self.into_iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<T> {
        self.into_iter()
    }
}

impl<T> Default for HashSet<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, T> IntoIterator for &'a HashSet<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        Iter {
            inner: self.map.iter(),
        }
    }
}

impl<'a, T> IntoIterator for &'a mut HashSet<T> {
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        IterMut {
            inner: self.map.iter_mut(),
        }
    }
}

impl<T> IntoIterator for HashSet<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            inner: self.map.into_iter(),
        }
    }
}

impl<T> HashSet<T>
where
    T: Hash + Eq,
{
    pub fn get<'a>(&'a self, key: &T) -> Option<&'a T> {
        self.map.get_kv(key).map(|(k, ())| k)
    }

    pub fn get_mut<'a>(&'a mut self, key: &T) -> Option<&'a mut T> {
        self.map.get_mut_kv(key).map(|(k, ())| k)
    }

    pub fn insert(&mut self, key: T) -> Option<T> {
        self.map.insert_kv(key, ()).map(|(k, ())| k)
    }

    pub fn reserve(&mut self, additional: usize) {
        self.map.reserve_exact(additional)
    }

    pub fn reserve_exact(&mut self, additional: usize) {
        self.map.reserve_exact(additional);
    }

    pub fn shrink_to(&mut self, min_capacity: usize) {
        self.map.shrink_to(min_capacity);
    }

    pub fn shrink_to_fit(&mut self) {
        self.map.shrink_to_fit();
    }
}

#[derive(Clone)]
pub struct Iter<'a, T> {
    inner: hash_map::Iter<'a, T, ()>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(k, _)| k)
    }
}

pub struct IterMut<'a, T> {
    inner: hash_map::IterMut<'a, T, ()>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(k, _)| k)
    }
}

#[derive(Clone)]
pub struct IntoIter<T> {
    inner: hash_map::IntoIter<T, ()>,
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(k, _)| k)
    }
}
