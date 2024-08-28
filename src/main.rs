mod hashmap;
use hashmap::HashMap;

fn main() {
    let mut map: HashMap<&str, &str> = HashMap::new();
    map.insert("hello", "你好");
    map.insert("world", "世界");
    println!("{:?}", map.get(&"hello"));
    println!("{:?}", map.get(&"world"));
    println!("{:?}", map.get(&"abcdefg"));
}
