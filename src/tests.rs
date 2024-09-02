#![cfg(test)]

#[allow(unused_imports)]
use super::hash_map::*;
#[allow(unused_imports)]
use std::hash::{Hash, Hasher};

#[test]
fn basics() {
    let mut map: HashMap<&str, &str> = HashMap::new();
    map.insert("hello", "你好");
    map.insert("world", "世界");
    assert_eq!(map.get(&"hello"), Some(&"你好"));
    assert_eq!(map.get(&"world"), Some(&"世界"));
    assert_eq!(map.get(&"abcdefg"), None);
    assert_eq!(map.len(), 2);
    map.remove(&"world");
    assert_eq!(map.get(&"world"), None);
    assert_eq!(map.len(), 1);
}

#[test]
fn rehash() {
    let mut map: HashMap<i32, i32> = HashMap::with_capacity(2);
    assert_eq!(map.capacity(), 2);
    map.insert(10, 10);
    map.resize(32);
    assert_eq!(map.capacity(), 32);
    map.insert(20, 20);
    map.insert(30, 30);
    map.resize(16);
    assert_eq!(map.capacity(), 16);
    assert_eq!(map.get(&10), Some(&10));
    assert_eq!(map.get(&20), Some(&20));
    assert_eq!(map.get(&30), Some(&30));
}

#[test]
fn hash_collision() {
    #[derive(PartialEq, Eq)]
    struct Thing(u8);
    impl Hash for Thing {
        fn hash<H: Hasher>(&self, state: &mut H) {
            0u64.hash(state);
        }
    }
    let mut map: HashMap<Thing, u8> = HashMap::new();
    map.insert(Thing(0), 10);
    map.insert(Thing(1), 20);
    assert_eq!(map.get(&Thing(0)), Some(&10));
    assert_eq!(map.get(&Thing(1)), Some(&20));
    map.resize(32);
    assert_eq!(map.capacity(), 32);
    assert_eq!(map.get(&Thing(0)), Some(&10));
    assert_eq!(map.get(&Thing(1)), Some(&20));
    map.resize(2);
    assert_eq!(map.capacity(), 2);
    assert_eq!(map.get(&Thing(0)), Some(&10));
    assert_eq!(map.get(&Thing(1)), Some(&20));
    map.remove(&Thing(0));
    assert_eq!(map.get(&Thing(0)), None);
    assert_eq!(map.get(&Thing(1)), Some(&20));
}

#[test]
fn everything() {
    let mut map: HashMap<i32, i32> = HashMap::new();
    for i in 0..100 {
        map.insert(i, i * 2);
    }
    for i in 0..100 {
        assert_eq!(map.get(&i), Some(&(i * 2)));
    }
    for i in 20..80 {
        assert!(map.remove(&i).is_some());
    }
    map.shrink_to_fit();
    for i in 0..20 {
        assert_eq!(map.get(&i), Some(&(i * 2)));
    }
    for i in 80..100 {
        assert_eq!(map.get(&i), Some(&(i * 2)));
    }
}

#[test]
fn zst() {
    let mut map: HashMap<(), ()> = HashMap::new();
    assert_eq!(map.capacity(), isize::MAX as usize);
    assert_eq!(map.get(&()), None);
    map.insert((), ());
    assert_eq!(map.get(&()), Some(&()));
}

#[test]
fn iter() {
    let mut map: HashMap<i32, i32> = HashMap::new();
    for i in 0..10 {
        map.insert(i, i * 2);
    }
    map.resize(9); // to make sure a bucket holds more than one elements

    // Non-mut borrowing iterator.
    let mut pairs: Vec<(&i32, &i32)> = map.iter().collect();
    pairs.sort_by(|(k0, _), (k1, _)| k0.cmp(k1)); // because hash map is unordered.
    for i in 0..10 {
        assert_eq!(pairs[i as usize], (&i, &(i * 2)));
    }

    // Mut borrowing iterator.
    let mut pairs_mut: Vec<(&mut i32, &mut i32)> = map.iter_mut().collect();
    pairs_mut.sort_by(|(k0, _), (k1, _)| k0.cmp(k1));
    for i in 0..10 {
        let (mut k, mut v) = (i, i * 2);
        assert_eq!(pairs_mut[i as usize], (&mut k, &mut v));
    }

    // Owning iterator.
    let mut into_pairs: Vec<(i32, i32)> = map.into_iter().collect();
    into_pairs.sort_by(|(k0, _), (k1, _)| k0.cmp(k1));
    for i in 0..10 {
        assert_eq!(into_pairs[i as usize], (i, i * 2));
    }
}
