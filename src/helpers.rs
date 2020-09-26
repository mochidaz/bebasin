use std::collections::{HashMap, HashSet};

pub trait AppendableMap<K: std::cmp::Eq + std::hash::Hash, V> {
    fn append(&mut self, other: HashMap<K, HashSet<V>>) -> Result<(), ()>;
}

impl<K: std::cmp::Eq + std::hash::Hash, V: std::cmp::Eq + std::hash::Hash> AppendableMap<K, V>
    for HashMap<K, HashSet<V>>
{
    fn append(&mut self, other: HashMap<K, HashSet<V>>) -> Result<(), ()> {
        for (key, value) in other {
            match self.get_mut(&key) {
                Some(old_val) => {
                    value.into_iter().for_each(|x| {
                        old_val.insert(x);
                    });
                }
                None => {
                    self.insert(key, value);
                }
            }
        }
        Ok(())
    }
}

