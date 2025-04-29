use hashbrown::hash_map as base;
use super::hasher::WrapHasher;

use core::hash::Hash;
use core::borrow::Borrow;


pub struct HashMap<K, V> {
    base: base::HashMap<K, V, WrapHasher>,
}

impl<K, V> HashMap<K, V> 
where 
    K: Eq + Hash,
{

    pub fn new() -> HashMap<K, V> {
        HashMap {
            base: base::HashMap::with_hasher(WrapHasher::new())
        }
    }

    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        self.base.insert(k, v)
    }

    pub fn iter(&self) -> base::Iter<'_, K, V> {
        self.base.iter()
    }

    pub fn len(&self) -> usize {
        self.base.len()
    }

    pub fn get<Q: ?Sized>(&self, k: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.base.get(k)
    }
}

impl<K, V> Eq for HashMap<K, V>
where
    K: Eq + Hash,
    V: Eq,
{
}

impl<K, V> PartialEq for HashMap<K, V>
where
    K: Eq + Hash,
    V: PartialEq,
{
    fn eq(&self, other: &HashMap<K, V>) -> bool {
        if self.len() != other.len() {
            return false;
        }

        self.iter().all(|(key, value)| other.get(key).map_or(false, |v| *value == *v))
    }
}