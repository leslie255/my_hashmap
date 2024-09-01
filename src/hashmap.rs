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
            unsafe { std::ptr::write_bytes(vec.as_mut_ptr(), 0, count) };
        }
        unsafe { vec.set_len(count) };
        vec
    }

    /// FIXME: Maybe make this into an iterator in the future.
    fn for_each_kv(self, mut f: impl FnMut(K, V)) {
        if let Option_::Some((k, v)) = self.first {
            f(k, v)
        }
        if let Some(others) = self.others.into_option() {
            for (k, v) in others {
                f(k, v);
            }
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
            _ => self
                .others
                .as_option()?
                .iter()
                .find(|(k0, _)| k0 == k)
                .map(|(_, v)| v),
        }
    }

    fn get_mut<'a>(&'a mut self, k: &K) -> Option<&'a mut V> {
        match &mut self.first {
            Option_::Some((k0, v)) if k == k0 => Some(v),
            _ => self
                .others
                .as_option_mut()?
                .iter_mut()
                .find(|(k0, _)| k0 == k)
                .map(|(_, v)| v),
        }
    }

    fn remove(&mut self, k: &K) -> Option<V> {
        match &mut self.first {
            Option_::Some((k0, _)) if k == k0 => {
                let (_, v) = mem::replace(&mut self.first, Option_::None).into_option()?;
                if let Option_::Some(vec) = &mut self.others {
                    self.first = vec.pop().into();
                    if vec.is_empty() {
                        self.others = Option_::None;
                    }
                }
                Some(v)
            }
            _ => {
                let others = self.others.as_option_mut()?;
                let idx = others.iter().position(|(k0, _)| k == k0)?;
                let (_, v) = others.remove(idx);
                Some(v)
            }
        }
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

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn capacity(&self) -> usize {
        if Self::is_zst() {
            isize::MAX as usize // to match behavior of `Vec` and `HashMap` in std
        } else {
            self.buckets.len()
        }
    }

    /// If `K` and `V` are both ZSTs.
    const fn is_zst() -> bool {
        K::IS_ZST && V::IS_ZST
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
        if self.buckets.is_empty() {
            self.resize(INIT_CAPACITY);
        } else if self.load_factor() > LOAD_FACTOR_MAX {
            self.resize(self.capacity() * 4);
        }
    }

    /// This function is `pub(crate)` for use in testing.
    /// # Panics
    /// Panics if `new_capacity == 0` and `self.len() != 0`.
    pub(crate) fn resize(&mut self, new_capacity: usize) {
        if Self::is_zst() {
            self.buckets = Bucket::vec_of_empties(new_capacity);
            return;
        }
        // FIXME: Realloc instead of rehashing into a new allocation?
        let old_buckets: Vec<Bucket<K, V>> = {
            let mut buckets = Bucket::vec_of_empties(new_capacity);
            mem::swap(&mut self.buckets, &mut buckets);
            buckets
        };
        if cfg!(debug_assertions) && new_capacity == 0 {
            // Only do this assertion in debug mode, because it would panic anyways later during
            // rehashing.
            assert!(
                self.is_empty(),
                "`HashMap::resize` called with `new_capacity = 0`, but `self.len() > 0`"
            );
        }
        for old_bucket in old_buckets {
            old_bucket.for_each_kv(|k, v| {
                self.bucket_mut(&k).unwrap().insert(k, v);
            });
        }
    }

    /// Hashes the key, mod the hash by the number of buckets.
    /// Returns `None` if capacity is zero.
    fn index(&self, key: &K) -> Option<usize> {
        let hash = hash(DefaultHasher::new(), key);
        (hash as usize).checked_rem(self.buckets.len())
    }

    /// The bucket for a key.
    /// Returns `None` if capacity is zero.
    fn bucket<'a>(&'a self, key: &K) -> Option<&'a Bucket<K, V>> {
        let idx = self.index(key)?;
        Some(&self.buckets[idx])
    }

    /// The bucket for a key.
    /// Returns `None` if capacity is zero.
    fn bucket_mut<'a>(&'a mut self, key: &K) -> Option<&'a mut Bucket<K, V>> {
        let idx = self.index(key)?;
        Some(&mut self.buckets[idx])
    }

    pub fn get<'a>(&'a self, key: &K) -> Option<&'a V> {
        self.bucket(key)?.get(key)
    }

    pub fn get_mut<'a>(&'a mut self, key: &K) -> Option<&'a mut V> {
        self.bucket_mut(key)?.get_mut(key)
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.len -= 1;
        self.bucket_mut(key)?.remove(key)
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.len += 1;
        self.expand_if_needed();
        self.bucket_mut(&key)?.insert(key, value).map(|(_, v)| v)
    }

    pub fn reserve(&mut self, additional: usize) {
        // FIXME: Reserve more aggressively here.
        self.reserve_exact(additional);
    }

    pub fn reserve_exact(&mut self, additional: usize) {
        let new_capacity = self.len() + additional;
        if self.capacity() < new_capacity {
            self.resize(new_capacity);
        }
    }

    pub fn shrink_to_fit(&mut self) {
        self.shrink_to(0)
    }

    pub fn shrink_to(&mut self, min_capacity: usize) {
        let needed_capacity = (self.len() as f64 / LOAD_FACTOR_MAX) as usize;
        self.resize(usize::max(needed_capacity, min_capacity));
    }
}

impl<K, V> Default for HashMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}
