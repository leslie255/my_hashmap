#![allow(dead_code)]

use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

const LOAD_FACTOR_MAX: f64 = 0.75;
const INIT_CAPACITY: usize = 32;

pub struct HashMap<K, V> {
    buckets: Vec<Bucket<K, V>>,
    len: usize,
}

fn hash(mut hasher: impl Hasher, x: impl Hash) -> u64 {
    x.hash(&mut hasher);
    hasher.finish()
}

#[derive(Debug, Clone)]
struct Bucket<K, V> {
    value: Option<(K, V)>,
}

impl<K, V> Default for Bucket<K, V> {
    fn default() -> Self {
        Self { value: None }
    }
}

impl<K, V> Bucket<K, V> {
    fn vec_of_empties(count: usize) -> Vec<Self> {
        let mut vec = Vec::with_capacity(count);
        if count != 0 {
            // FIXME: Maybe UB?
            unsafe {
                std::ptr::write_bytes(vec.as_mut_ptr(), 0, count);
                vec.set_len(count);
            }
        }
        vec
    }

    /// FIXME: Maybe make this into an iterator in the future.
    fn for_each_kv(self, mut f: impl FnMut(K, V)) {
        if let Some((k, v)) = self.value {
            f(k, v)
        }
    }
}

impl<K, V> Bucket<K, V>
where
    K: Eq,
{
    fn insert(&mut self, k: K, v: V) -> Option<V> {
        // FIXME: Handle hash collision.
        if self.value.as_ref().is_some_and(|(k_, _)| k_ != &k) {
            todo!("Handle hash collision");
        }
        let x = self.value.replace((k, v));
        x.map(|(_, v)| v)
    }

    fn get<'a>(&'a self, k: &K) -> Option<&'a V> {
        // FIXME: Handle hash collision.
        let (k_, v) = self.value.as_ref()?;
        assert!(k_ == k);
        Some(v)
    }
}

impl<K, V> HashMap<K, V> {
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buckets: Bucket::vec_of_empties(capacity),
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn capacity(&self) -> usize {
        self.buckets.len()
    }
}

impl<K, V> HashMap<K, V>
where
    K: Hash + Eq,
{
    fn load_factor(&self) -> f64 {
        (self.len() as f64) / (self.capacity() as f64)
    }

    /// If load_factor > LOAD_FACTOR_MAX or capacity is zero.
    fn need_expand(&self) -> bool {
        self.load_factor() > LOAD_FACTOR_MAX || self.capacity() == 0
    }

    /// Expand if `need_expand`.
    fn expand_if_needed(&mut self) {
        if self.need_expand() {
            let new_capacity = if self.capacity() == 0 {
                INIT_CAPACITY
            } else {
                self.capacity() * 4
            };
            self.resize(new_capacity);
        }
    }

    fn resize(&mut self, new_capacity: usize) {
        let old_buckets: Vec<Bucket<K, V>> = {
            let mut buckets = Bucket::vec_of_empties(new_capacity);
            std::mem::swap(&mut self.buckets, &mut buckets);
            buckets
        };
        for bucket in old_buckets {
            bucket.for_each_kv(|k, v| {
                let idx = self.index(&k).unwrap();
                self.buckets[idx].insert(k, v);
            });
        }
    }

    /// Hashes the key, returns an index.
    /// Returns `None` if capacity is zero.
    fn index(&self, key: &K) -> Option<usize> {
        let hash = hash(DefaultHasher::new(), key);
        (hash as usize).checked_rem(self.buckets.len())
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        let idx = self.index(key)?;
        self.buckets[idx].get(key)
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.expand_if_needed();
        // `unwrap` because `expand_if_needed` made sure that `capacity > 0`
        let idx = self.index(&key).unwrap();
        let bucket = &mut self.buckets[idx];
        self.len += 1;
        bucket.insert(key, value)
    }
}

impl<K, V> Default for HashMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basics() {
        let mut map: HashMap<&str, &str> = HashMap::new();
        map.insert("hello", "你好");
        map.insert("world", "世界");
        assert_eq!(map.get(&"hello"), Some(&"你好"));
        assert_eq!(map.get(&"world"), Some(&"世界"));
        assert_eq!(map.get(&"abcdefg"), None);
    }

    #[test]
    fn rehash() {
        let mut map: HashMap<&str, i32> = HashMap::with_capacity(2);
        map.insert("x", 255);
        map.resize(32);
        assert_eq!(map.capacity(), 32);
        assert_eq!(map.get(&"x"), Some(&255));
    }
}
