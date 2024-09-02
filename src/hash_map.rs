use std::{
    collections::hash_map::DefaultHasher,
    fmt::{self, Debug},
    hash::{Hash, Hasher},
    mem::{self, size_of},
    option, slice, vec,
};

const LOAD_FACTOR_MAX: f64 = 0.75;
const INIT_CAPACITY: usize = 32;

trait IsZst {
    const IS_ZST: bool;
}

impl<T> IsZst for T {
    const IS_ZST: bool = size_of::<Self>() == 0;
}

#[derive(Clone)]
pub struct HashMap<K, V> {
    buckets: Vec<Bucket<K, V>>,
    len: usize,
}

impl<K, V> Debug for HashMap<K, V>
where
    K: Debug,
    V: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_map().entries(self).finish()
    }
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

    fn iter(&self) -> BucketIter<K, V> {
        self.into_iter()
    }

    fn iter_mut(&mut self) -> BucketIterMut<K, V> {
        self.into_iter()
    }
}

impl<'a, K, V> IntoIterator for &'a Bucket<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = BucketIter<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter {
        BucketIter::new(
            self.first.as_option(),
            self.others
                .as_option()
                .map(Vec::as_slice)
                .unwrap_or_default(),
        )
    }
}

impl<'a, K, V> IntoIterator for &'a mut Bucket<K, V> {
    type Item = (&'a mut K, &'a mut V);
    type IntoIter = BucketIterMut<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter {
        BucketIterMut::new(
            self.first.as_option_mut(),
            self.others
                .as_option_mut()
                .map(Vec::as_mut_slice)
                .unwrap_or_default(),
        )
    }
}

impl<K, V> IntoIterator for Bucket<K, V> {
    type Item = (K, V);
    type IntoIter = BucketIntoIter<K, V>;
    fn into_iter(self) -> Self::IntoIter {
        BucketIntoIter::new(
            self.first.into_option(),
            self.others.into_option().unwrap_or_default(),
        )
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

    fn get<'a>(&'a self, k: &K) -> Option<(&'a K, &'a V)> {
        match &self.first {
            Option_::Some((k0, v)) if k == k0 => Some((k0, v)),
            _ => self
                .others
                .as_option()?
                .iter()
                .find(|(k0, _)| k0 == k)
                .map(|(k, v)| (k, v)),
        }
    }

    fn get_mut<'a>(&'a mut self, k: &K) -> Option<(&'a mut K, &'a mut V)> {
        match &mut self.first {
            Option_::Some((k0, v)) if k == k0 => Some((k0, v)),
            _ => self
                .others
                .as_option_mut()?
                .iter_mut()
                .find(|(k0, _)| k0 == k)
                .map(|(k, v)| (k, v)),
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

    pub fn iter(&self) -> Iter<K, V> {
        self.into_iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<K, V> {
        self.into_iter()
    }

    /// If `K` and `V` are both ZSTs.
    const fn is_zst() -> bool {
        K::IS_ZST && V::IS_ZST
    }
}

impl<'a, K, V> IntoIterator for &'a HashMap<K, V> {
    type Item = (&'a K, &'a V);

    type IntoIter = Iter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        Iter::new(&self.buckets)
    }
}

impl<'a, K, V> IntoIterator for &'a mut HashMap<K, V> {
    type Item = (&'a mut K, &'a mut V);

    type IntoIter = IterMut<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        IterMut::new(&mut self.buckets)
    }
}

impl<K, V> IntoIterator for HashMap<K, V> {
    type Item = (K, V);

    type IntoIter = IntoIter<K, V>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter::new(self.buckets)
    }
}

impl<K, V> Default for HashMap<K, V> {
    fn default() -> Self {
        Self::new()
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

    pub fn get_kv<'a>(&'a self, key: &K) -> Option<(&'a K, &'a V)> {
        self.bucket(key)?.get(key)
    }

    pub fn get<'a>(&'a self, key: &K) -> Option<&'a V> {
        self.get_kv(key).map(|(_, v)| v)
    }

    pub fn get_mut_kv<'a>(&'a mut self, key: &K) -> Option<(&'a mut K, &'a mut V)> {
        self.bucket_mut(key)?.get_mut(key)
    }

    pub fn get_mut<'a>(&'a mut self, key: &K) -> Option<&'a mut V> {
        self.get_mut_kv(key).map(|(_, v)| v)
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.len -= 1;
        self.bucket_mut(key)?.remove(key)
    }

    pub fn insert_kv(&mut self, key: K, value: V) -> Option<(K, V)> {
        self.len += 1;
        self.expand_if_needed();
        self.bucket_mut(&key)?.insert(key, value)
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.insert_kv(key, value).map(|(_, v)| v)
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

#[derive(Clone)]
struct BucketIter<'a, K, V> {
    first: option::IntoIter<&'a (K, V)>,
    others: slice::Iter<'a, (K, V)>,
}

impl<'a, K, V> BucketIter<'a, K, V> {
    fn new(first: Option<&'a (K, V)>, others: &'a [(K, V)]) -> Self {
        Self {
            first: first.into_iter(),
            others: others.iter(),
        }
    }
}

impl<'a, K, V> Iterator for BucketIter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((k, v)) = self.first.next() {
            return Some((k, v));
        }
        self.others.next().map(|(k, v)| (k, v))
    }
}

struct BucketIterMut<'a, K, V> {
    first: option::IntoIter<&'a mut (K, V)>,
    others: slice::IterMut<'a, (K, V)>,
}

impl<'a, K, V> BucketIterMut<'a, K, V> {
    fn new(first: Option<&'a mut (K, V)>, others: &'a mut [(K, V)]) -> Self {
        Self {
            first: first.into_iter(),
            others: others.iter_mut(),
        }
    }
}

impl<'a, K, V> Iterator for BucketIterMut<'a, K, V> {
    type Item = (&'a mut K, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((k, v)) = self.first.next() {
            return Some((k, v));
        }
        self.others.next().map(|(k, v)| (k, v))
    }
}

#[derive(Clone)]
struct BucketIntoIter<K, V> {
    first: option::IntoIter<(K, V)>,
    others: vec::IntoIter<(K, V)>,
}

impl<K, V> BucketIntoIter<K, V> {
    fn new(first: Option<(K, V)>, others: Vec<(K, V)>) -> Self {
        Self {
            first: first.into_iter(),
            others: others.into_iter(),
        }
    }
}

impl<K, V> Iterator for BucketIntoIter<K, V> {
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((k, v)) = self.first.next() {
            return Some((k, v));
        }
        self.others.next()
    }
}

#[derive(Clone)]
pub struct Iter<'a, K, V> {
    buckets: slice::Iter<'a, Bucket<K, V>>,
    current_bucket: Option<BucketIter<'a, K, V>>,
}

impl<'a, K, V> Iter<'a, K, V> {
    fn new(buckets: &'a [Bucket<K, V>]) -> Self {
        Self {
            buckets: buckets.iter(),
            current_bucket: None,
        }
    }
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match &mut self.current_bucket {
                Some(bucket_iter) => match bucket_iter.next() {
                    Some(kv) => break Some(kv),
                    None => {
                        self.current_bucket = self.buckets.next().map(Bucket::iter);
                        continue;
                    }
                },
                None => {
                    self.current_bucket = Some(self.buckets.next().map(Bucket::iter)?);
                    continue;
                }
            }
        }
    }
}

pub struct IterMut<'a, K, V> {
    buckets: slice::IterMut<'a, Bucket<K, V>>,
    current_bucket: Option<BucketIterMut<'a, K, V>>,
}

impl<'a, K, V> IterMut<'a, K, V> {
    fn new(buckets: &'a mut [Bucket<K, V>]) -> Self {
        Self {
            buckets: buckets.iter_mut(),
            current_bucket: None,
        }
    }
}

impl<'a, K, V> Iterator for IterMut<'a, K, V> {
    type Item = (&'a mut K, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match &mut self.current_bucket {
                Some(bucket_iter) => match bucket_iter.next() {
                    Some(kv) => break Some(kv),
                    None => {
                        self.current_bucket = self.buckets.next().map(Bucket::iter_mut);
                        continue;
                    }
                },
                None => {
                    self.current_bucket = Some(self.buckets.next().map(Bucket::iter_mut)?);
                    continue;
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct IntoIter<K, V> {
    buckets: vec::IntoIter<Bucket<K, V>>,
    current_bucket: Option<BucketIntoIter<K, V>>,
}

impl<K, V> IntoIter<K, V> {
    fn new(buckets: Vec<Bucket<K, V>>) -> Self {
        Self {
            buckets: buckets.into_iter(),
            current_bucket: None,
        }
    }
}

impl<K, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match &mut self.current_bucket {
                Some(bucket_iter) => match bucket_iter.next() {
                    Some(kv) => break Some(kv),
                    None => {
                        self.current_bucket = self.buckets.next().map(Bucket::into_iter);
                        continue;
                    }
                },
                None => {
                    self.current_bucket = Some(self.buckets.next().map(Bucket::into_iter)?);
                    continue;
                }
            }
        }
    }
}
