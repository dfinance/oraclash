use crate::shm::Memory;
use crate::sorted_set::{Binary, BinaryCmp, Cmp, SetIterator, SortedSet};
use std::cmp::Ordering;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::marker::PhantomData;

/// Fixed size elements shared memory map.
pub struct ShmMap<'a, K, V>
where
    K: Binary + BinaryCmp,
    V: Binary,
{
    set: SortedSet<'a, Entry<K, V>>,
}

impl<'a, K, V> ShmMap<'a, K, V>
where
    K: Binary + BinaryCmp,
    V: Binary,
{
    /// Create a new shared memory map with given memory reference.
    pub fn new(memory: Memory<'a>) -> ShmMap<'a, K, V> {
        let set = SortedSet::new(memory);
        ShmMap { set }
    }

    /// Put a value to the map.
    pub fn put(&mut self, key: K, value: V) {
        self.set.add(Entry { key, value });
    }

    /// Retrieve the value associated with the key.
    pub fn get(&self, key: K) -> Option<V> {
        let mut buffer = vec![0; K::const_size() as usize];
        key.to_bytes(&mut buffer);

        self.set.find(KeyCmp::new(key)).map(|e| e.value)
    }

    /// Removes all keys.
    pub fn clear(&mut self) {
        self.set.clear();
    }

    /// Create map iterator.
    pub fn iter(&self) -> MapIterator<'_, '_, K, V> {
        MapIterator {
            iter: self.set.iter(),
        }
    }
}

/// Map iterator.
pub struct MapIterator<'a, 'b, K, V>
where
    K: Binary + BinaryCmp,
    V: Binary,
{
    iter: SetIterator<'a, 'b, Entry<K, V>>,
}

impl<'a, 'b, K, V> Iterator for MapIterator<'a, 'b, K, V>
where
    K: Binary + BinaryCmp,
    V: Binary,
{
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|e| (e.key, e.value))
    }
}

impl<'a, K, V> Display for ShmMap<'a, K, V>
where
    K: Binary + BinaryCmp + Display,
    V: Binary + BinaryCmp + Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        for item in self.iter() {
            write!(f, "({}->{}),", item.0, item.1)?;
        }
        write!(f, "]")
    }
}

struct KeyCmp<K>
where
    K: Binary + BinaryCmp,
{
    key: Vec<u8>,
    _type: PhantomData<K>,
}

impl<K> KeyCmp<K>
where
    K: Binary + BinaryCmp,
{
    pub fn new(key: K) -> KeyCmp<K> {
        let mut buffer = vec![0; K::const_size() as usize];
        key.to_bytes(&mut buffer);

        KeyCmp {
            key: buffer,
            _type: PhantomData,
        }
    }
}

impl<K> Cmp for KeyCmp<K>
where
    K: Binary + BinaryCmp,
{
    fn cmp(&self, order: &[u8]) -> Ordering {
        K::cmp(&self.key, &order[0..K::const_size() as usize])
    }
}

pub struct Entry<K, V> {
    pub key: K,
    pub value: V,
}

impl<K, V> Binary for Entry<K, V>
where
    K: Binary,
    V: Binary,
{
    fn const_size() -> u32 {
        K::const_size() + V::const_size()
    }

    fn to_bytes(&self, buffer: &mut [u8]) {
        self.key.to_bytes(&mut buffer[0..K::const_size() as usize]);
        self.value.to_bytes(
            &mut buffer
                [K::const_size() as usize..K::const_size() as usize + V::const_size() as usize],
        );
    }

    fn from_bytes(buffer: &[u8]) -> Self {
        Entry {
            key: K::from_bytes(&buffer[0..K::const_size() as usize]),
            value: V::from_bytes(
                &buffer
                    [K::const_size() as usize..K::const_size() as usize + V::const_size() as usize],
            ),
        }
    }
}

impl<K, V> BinaryCmp for Entry<K, V>
where
    K: Binary + BinaryCmp,
    V: Binary,
{
    fn cmp(left: &[u8], right: &[u8]) -> Ordering {
        K::cmp(
            &left[0..K::const_size() as usize],
            &right[0..K::const_size() as usize],
        )
    }
}
