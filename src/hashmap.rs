use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    mem::{self, size_of},
};

const LOAD_FACTOR_MAX: f64 = 0.75;
const INIT_CAPACITY: usize = 32;

trait IsZst {
    const IS_ZST: bool;
}

impl<T> IsZst for T {
    const IS_ZST: bool = size_of::<Self>() == 0;
}

pub struct HashMap<K, V> {
    buckets: Vec<Bucket<K, V>>,
    len: usize,
}

fn hash(mut hasher: impl Hasher, x: impl Hash) -> u64 {
    x.hash(&mut hasher);
    hasher.finish()
}

/// `Option` type with no niche value optimization and can be initialized as `None` by zeros in
/// memory.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Option_<T> {
    None = 0,
    Some(T),
}

impl<T> Default for Option_<T> {
    fn default() -> Self {
        Self::None
    }
}

#[allow(dead_code)]
impl<T> Option_<T> {
    fn into_option(self) -> Option<T> {
        match self {
            Option_::Some(x) => Some(x),
            Option_::None => None,
        }
    }
    const fn as_option(&self) -> Option<&T> {
        match self {
            Option_::Some(x) => Some(x),
            Option_::None => None,
        }
    }
    fn as_option_mut(&mut self) -> Option<&mut T> {
        match self {
            Option_::Some(x) => Some(x),
            Option_::None => None,
        }
    }
}

impl<T> From<Option<T>> for Option_<T> {
    fn from(value: Option<T>) -> Self {
        match value {
            Some(x) => Self::Some(x),
            None => Self::None,
        }
    }
}

/// `Bucket`'s default value is made from all zeros in memory.
#[derive(Debug, Clone)]
struct Bucket<K, V> {
    first: Option_<(K, V)>,
    others: Option_<Vec<(K, V)>>,
}

impl<K, V> Default for Bucket<K, V> {
    fn default() -> Self {
        Self {
            first: Option_::None,
            others: Option_::None,
        }
    }
}

impl<K, V> Bucket<K, V> {
    fn vec_of_empties(count: usize) -> Vec<Self> {
        let mut vec = Vec::with_capacity(count);
        if count != 0 && (!K::IS_ZST && !V::IS_ZST) {
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
        if let Option_::Some((k, v)) = self.first {
            f(k, v)
        }
    }
}

impl<K, V> Bucket<K, V>
where
    K: Eq,
{
    fn insert(&mut self, k: K, v: V) -> Option<(K, V)> {
        match &mut self.first {
            first @ Option_::None => {
                *first = Option_::Some((k, v));
                None
            }
            Option_::Some((ref k0, _)) if k0 == &k => {
                mem::replace(&mut self.first, Option_::Some((k, v))).into_option()
            }
            Option_::Some(_) => {
                let others = match &mut self.others {
                    Option_::Some(others) => others,
                    others @ Option_::None => {
                        *others = Option_::Some(Vec::with_capacity(1));
                        // Safety: Was just set as Some.
                        unsafe { others.as_option_mut().unwrap_unchecked() }
                    }
                };
                others.push((k, v));
                None
            }
        }
    }

    fn get<'a>(&'a self, k: &K) -> Option<&'a V> {
        match &self.first {
            Option_::Some((k0, v)) if k == k0 => Some(v),
            _ => self.find_in_others(k),
        }
    }

    fn find_in_others<'a>(&'a self, k: &K) -> Option<&'a V> {
        self.others
            .as_option()
            .as_ref()?
            .iter()
            .find_map(|(k0, v)| if k0 == k { Some(v) } else { None })
    }

    fn get_mut<'a>(&'a mut self, _k: &K) -> Option<&'a mut V> {
        todo!()
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
        if K::IS_ZST && V::IS_ZST {
            isize::MAX as usize // to match behavior of `Vec` and `HashMap` in std
        } else {
            self.buckets.len()
        }
    }
}

impl<K, V> HashMap<K, V>
where
    K: Hash + Eq,
{
    fn load_factor(&self) -> f64 {
        (self.len() as f64) / (self.capacity() as f64)
    }

    fn expand_if_needed(&mut self) {
        let need_expand = if K::IS_ZST && V::IS_ZST {
            false
        } else {
            self.load_factor() > LOAD_FACTOR_MAX || self.capacity() == 0
        };
        if need_expand {
            let new_capacity = if self.capacity() == 0 {
                INIT_CAPACITY
            } else {
                self.capacity() * 4
            };
            self.resize(new_capacity);
        }
    }

    pub(crate) fn resize(&mut self, new_capacity: usize) {
        if K::IS_ZST && V::IS_ZST {
            return;
        }
        let old_buckets: Vec<Bucket<K, V>> = {
            let mut buckets = Bucket::vec_of_empties(new_capacity);
            mem::swap(&mut self.buckets, &mut buckets);
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

    pub fn get<'a>(&'a self, key: &K) -> Option<&'a V> {
        let idx = self.index(key)?;
        self.buckets[idx].get(key)
    }

    pub fn get_mut<'a>(&'a mut self, key: &K) -> Option<&'a mut V> {
        let idx = self.index(key)?;
        self.buckets[idx].get_mut(key)
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.expand_if_needed();
        // `unwrap` because `expand_if_needed` made sure that `capacity > 0`.
        let idx = self.index(&key).unwrap();
        let bucket = &mut self.buckets[idx];
        self.len += 1;
        bucket.insert(key, value).map(|(_, v)| v)
    }

    pub fn reserve(&mut self, additional: usize) {
        self.reserve_exact(additional);
    }

    pub fn reserve_exact(&mut self, additional: usize) {
        let new_capacity = self.len() + additional;
        if self.capacity() < new_capacity {
            self.resize(new_capacity);
        }
    }

    pub fn shrink_to(&mut self, min_capacity: usize) {
        let needed_capacity = (self.len() as f64 / LOAD_FACTOR_MAX) as usize;
        self.resize(usize::min(needed_capacity, min_capacity));
    }
}

impl<K, V> Default for HashMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}
