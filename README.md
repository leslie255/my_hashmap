# Leslie255's Hash Map Implementation

**My own hash map implementation for fun**

```rs
use hashmap::HashMap;

fn main() {
  let mut map: HashMap<&str, &str> = HashMap::new();
  map.insert("hello", "你好");
  map.insert("world", "世界");
  assert_eq!(map.get(&"hello"), Some(&"你好"));
  assert_eq!(map.get(&"world"), Some(&"世界"));
  assert_eq!(map.get(&"abcde"), None);
}
```

## Progress

- [x] Insert
- [x] Find
- [x] Re-alloc / Rehash
- [x] Hash collision handle
- [x] ZST
- [x] Remove
- [x] Iterate
