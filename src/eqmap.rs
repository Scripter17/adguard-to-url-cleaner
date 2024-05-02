pub struct EqMap<K: Eq, V> {
    keys: Vec<K>,
    values: Vec<V>
}

impl<K: Eq, V> EqMap<K, V> {
    pub fn get(&self, key: &K) -> Option<&V> {
        for (i, x) in self.keys.iter().enumerate() {
            if x==key {
                return Some(&self.values[i]);
            }
        }
        None
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        for (i, x) in self.keys.iter().enumerate() {
            if x==key {
                return Some(&mut self.values[i]);
            }
        }
        None
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        for (i, x) in self.keys.iter().enumerate() {
            if *x==key {
                return Some(std::mem::replace(&mut self.values[i], value));
            }
        }
        self.keys.push(key);
        self.values.push(value);
        None
    }
}
