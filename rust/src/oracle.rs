use crate::map::ShmMap;
use crate::shm::Memory;
use crate::sorted_set::{Binary, BinaryCmp, LEN_SIZE};
use byteorder::{ByteOrder, LittleEndian};
use std::cmp::Ordering;
use std::hash::Hasher;
use twox_hash::XxHash64;

/// Currency pair ticker.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialOrd, PartialEq)]
pub struct Ticker(u64);

impl Ticker {
    /// Create a ticker with string.
    pub fn new(ticker: &str) -> Ticker {
        Ticker(str_xxhash(&ticker.to_ascii_lowercase()))
    }
}

/// Currency pair price.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Price(pub u64);

/// Price oracle cache.
pub struct PriceOracleCache<'a> {
    cache: ShmMap<'a, Ticker, Price>,
}

impl<'a> PriceOracleCache<'a> {
    /// Returns memory layout size for the given oracle pairs count.
    pub fn calculate_memory_size(pair_count: usize) -> usize {
        (Ticker::const_size() + Price::const_size()) as usize * pair_count + LEN_SIZE
    }

    /// Create a new shared memory map with given memory reference.
    pub fn new(memory: Memory<'a>) -> PriceOracleCache<'a> {
        PriceOracleCache {
            cache: ShmMap::new(memory),
        }
    }

    /// Put or update oracle ticker->price pair.
    pub fn put(&mut self, ticker: Ticker, price: Price) {
        self.cache.put(ticker, price)
    }

    /// Return price by ticker.
    pub fn get(&self, ticker: Ticker) -> Option<Price> {
        self.cache.get(ticker)
    }

    /// Clear oracle cache.
    pub fn clear(&mut self) {
        self.cache.clear();
    }
}

impl Binary for Ticker {
    fn const_size() -> u32 {
        8
    }

    fn to_bytes(&self, buffer: &mut [u8]) {
        LittleEndian::write_u64(&mut buffer[0..8], self.0);
    }

    fn from_bytes(buffer: &[u8]) -> Self {
        Ticker(LittleEndian::read_u64(&buffer[0..8]))
    }
}

impl BinaryCmp for Ticker {
    fn cmp(left: &[u8], right: &[u8]) -> Ordering {
        Ticker::from_bytes(left).cmp(&Ticker::from_bytes(right))
    }
}

impl Binary for Price {
    fn const_size() -> u32 {
        8
    }

    fn to_bytes(&self, buffer: &mut [u8]) {
        LittleEndian::write_u64(&mut buffer[0..8], self.0);
    }

    fn from_bytes(buffer: &[u8]) -> Self {
        Price(LittleEndian::read_u64(&buffer[0..8]))
    }
}

/// Calculate string hash.
fn str_xxhash(val: &str) -> u64 {
    let mut hash = XxHash64::default();
    Hasher::write(&mut hash, val.as_bytes());
    Hasher::finish(&hash)
}

#[cfg(test)]
mod tests {
    use crate::oracle::{Price, PriceOracleCache, Ticker};
    use crate::shm::Shm;

    #[test]
    fn test_oracle() {
        let shm = Shm::open_or_create(
            "/test_oracle",
            PriceOracleCache::calculate_memory_size(10) as u32,
        )
        .unwrap();
        let mut oracle = PriceOracleCache::new(shm.memory());
        oracle.clear();
        oracle.put(Ticker::new("BTCUSD"), Price(8000));
        oracle.put(Ticker::new("USDRUB"), Price(70));
        assert_eq!(Some(Price(70)), oracle.get(Ticker::new("USDRUB")));
        assert_eq!(Some(Price(8000)), oracle.get(Ticker::new("BTCuSD")));
        assert_eq!(None, oracle.get(Ticker::new("BTCR")));
    }
}
