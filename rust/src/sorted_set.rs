use crate::shm::Memory;
use byteorder::{ByteOrder, LittleEndian};
use std::cmp::Ordering;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::marker::PhantomData;
use std::ops::Range;

/// Set length size.
pub const LEN_SIZE: usize = 4;
/// Length bytes range;
pub const SIZE_RANGE: Range<usize> = 0..LEN_SIZE;

/// Sorted set shared memory implementation.
pub struct SortedSet<'a, T: Binary + BinaryCmp> {
    mem: Memory<'a>,
    _item: PhantomData<T>,
}

impl<'a, T> SortedSet<'a, T>
where
    T: Binary + BinaryCmp,
{
    /// Create a new sorted set with the given memory reference.
    pub fn new(memory: Memory<'a>) -> SortedSet<'a, T> {
        SortedSet {
            mem: memory,
            _item: PhantomData,
        }
    }

    /// Removes all elements from the set.
    pub fn clear(&mut self) {
        self.set_len(0);
    }

    /// Add new elements to the set.
    pub fn add(&mut self, item: T) {
        let mut buffer = vec![0; T::const_size() as usize];
        item.to_bytes(&mut buffer);
        let cmp: Comparator<T> = Comparator {
            buffer: &buffer,
            t: PhantomData,
        };

        match self.find_index(cmp) {
            Find::Index(index) => {
                self.store_at_index(index, &buffer);
            }
            Find::LastRange((start, _)) => {
                self.shift_right(start);
                self.store_at_index(start, &buffer);
                self.set_len(self.len() + 1);
            }
        }
    }

    /// Get the element by the given index.
    pub fn get(&self, index: usize) -> Option<T> {
        let len = self.len();
        if index >= len {
            None
        } else {
            Some(T::from_bytes(self.get_by_index(index)))
        }
    }

    /// Returns set length.
    pub fn len(&self) -> usize {
        LittleEndian::read_u32(&self.mem.mem_ref()[SIZE_RANGE]) as usize
    }

    /// Returns `true` if the set contains no elements.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Create vector with set elements.
    pub fn to_vec(&self) -> Vec<T> {
        self.iter().collect()
    }

    /// Finds the element by given comparator.
    pub fn find(&self, order: impl Cmp) -> Option<T> {
        match self.find_index(order) {
            Find::Index(index) => Some(T::from_bytes(self.get_by_index(index))),
            Find::LastRange(_) => None,
        }
    }

    /// Returns iterator.
    pub fn iter(&self) -> SetIterator<T> {
        SetIterator {
            set: &self,
            index: 0,
        }
    }

    /// Shifts all elements starting from the index to the right.
    fn shift_right(&mut self, index: usize) {
        let size = T::const_size() as usize;
        for index in (index..self.len()).rev() {
            let current_offset = self.offset(index);
            let next_offset = current_offset + size as usize;
            let mem = self.mem.mem_ref_mut();
            let (left, right) = mem.split_at_mut(next_offset);
            right[0..size].copy_from_slice(&left[current_offset..next_offset]);
        }
    }

    /// Stores bytes by index.
    fn store_at_index(&mut self, index: usize, buffer: &[u8]) {
        let offset = self.offset(index);
        let rf = &mut self.mem.mem_ref_mut()[offset..offset + T::const_size() as usize];
        rf.copy_from_slice(buffer);
    }

    /// Find the element index or or the closest interval.
    fn find_index(&self, order: impl Cmp) -> Find {
        let len = self.len();
        if len == 0 {
            return Find::LastRange((0, 0));
        }

        let mut list = (0, len);
        loop {
            let middle = (list.0 + list.1) / 2;
            list = match order.cmp(self.get_by_index(middle)) {
                Ordering::Equal => return Find::Index(middle),
                Ordering::Less => (list.0, middle),
                Ordering::Greater => (middle + 1, list.1),
            };
            if list.0 >= list.1 {
                break;
            }
        }

        Find::LastRange(list)
    }

    /// Get bytes by index.
    fn get_by_index(&self, index: usize) -> &[u8] {
        let offset = self.offset(index);
        &self.mem.mem_ref()[offset..offset + T::const_size() as usize]
    }

    /// Create element offset by its index.
    fn offset(&self, index: usize) -> usize {
        T::const_size() as usize * index + LEN_SIZE
    }

    /// Set list length.
    fn set_len(&mut self, len: usize) {
        LittleEndian::write_u32(&mut self.mem.mem_ref_mut()[SIZE_RANGE], len as u32);
    }
}

struct Comparator<'a, T: Binary + BinaryCmp> {
    buffer: &'a [u8],
    t: PhantomData<T>,
}

impl<'a, T> Cmp for Comparator<'a, T>
where
    T: Binary + BinaryCmp,
{
    fn cmp(&self, order: &[u8]) -> Ordering {
        T::cmp(self.buffer, order)
    }
}

enum Find {
    Index(usize),
    LastRange((usize, usize)),
}

/// List element trait.
pub trait Binary {
    fn const_size() -> u32;
    fn to_bytes(&self, buffer: &mut [u8]);
    fn from_bytes(buffer: &[u8]) -> Self;
}

/// Binary comparator.
pub trait BinaryCmp {
    fn cmp(left: &[u8], right: &[u8]) -> Ordering;
}

/// Binary comparator.
pub trait Cmp {
    fn cmp(&self, order: &[u8]) -> Ordering;
}

pub struct SetIterator<'a, 'b, T>
where
    T: Binary + BinaryCmp,
{
    set: &'b SortedSet<'a, T>,
    index: usize,
}

impl<T> Iterator for SetIterator<'_, '_, T>
where
    T: Binary + BinaryCmp,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.set.get(self.index);
        self.index += 1;
        value
    }
}

impl<'a, T> Display for SortedSet<'a, T>
where
    T: Binary + BinaryCmp + Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        for item in self.iter() {
            write!(f, "{},", item)?;
        }
        write!(f, "]")
    }
}

#[cfg(test)]
mod tests {
    use crate::shm::Shm;
    use crate::sorted_set::{Binary, BinaryCmp, SortedSet};
    use byteorder::{ByteOrder, LittleEndian};
    use std::cmp::Ordering;
    use std::fmt;
    use std::fmt::Formatter;

    #[test]
    fn test_list() {
        let shm = Shm::open_or_create("/test", 1024).unwrap();
        let mut set = SortedSet::new(shm.memory());
        set.clear();
        set.add(Pair::new(1, 2));
        set.add(Pair::new(3, 1));
        set.add(Pair::new(0, 10));
        set.add(Pair::new(100, 100));
        set.add(Pair::new(10, 0));
        set.add(Pair::new(11, 0));
        set.add(Pair::new(11, 44));
        set.add(Pair::new(2, 2));

        assert_eq!(
            vec![
                Pair::new(0, 10),
                Pair::new(1, 2),
                Pair::new(2, 2),
                Pair::new(3, 1),
                Pair::new(10, 0),
                Pair::new(11, 44),
                Pair::new(100, 100),
            ],
            set.to_vec()
        );
    }

    #[derive(Ord, PartialOrd, Eq, PartialEq, Debug)]
    struct Pair {
        key: u64,
        value: u64,
    }

    impl Pair {
        pub fn new(key: u64, value: u64) -> Pair {
            Pair { key, value }
        }
    }

    impl Binary for Pair {
        fn const_size() -> u32 {
            16
        }

        fn to_bytes(&self, buffer: &mut [u8]) {
            LittleEndian::write_u64(&mut buffer[0..8], self.key);
            LittleEndian::write_u64(&mut buffer[8..16], self.value);
        }

        fn from_bytes(buffer: &[u8]) -> Self {
            Pair {
                key: LittleEndian::read_u64(&buffer[0..8]),
                value: LittleEndian::read_u64(&buffer[8..16]),
            }
        }
    }

    impl BinaryCmp for Pair {
        fn cmp(left: &[u8], right: &[u8]) -> Ordering {
            let left = LittleEndian::read_u64(&left[0..8]);
            let right = LittleEndian::read_u64(&right[0..8]);
            left.cmp(&right)
        }
    }

    impl fmt::Display for Pair {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            write!(f, "({}:{})", self.key, self.value)
        }
    }
}
