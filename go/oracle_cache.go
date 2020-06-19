package _go

import (
	"encoding/binary"
	"fmt"
	"github.com/cespare/xxhash"
	"strings"
	"vm/shm"
)

const LenSize uint32 = 4
const ItemSize uint32 = 16
const TickerSize uint32 = 8

type OracleCache struct {
	mem      *shm.Memory
	capacity uint32
}

func CreateOracleCache(name string, capacity uint32) (*OracleCache, error) {
	bufferSize := int32(LenSize + (capacity * ItemSize))
	mem, err := shm.Open(name, bufferSize)
	if err != nil {
		mem, err = shm.Create(name, bufferSize)
		if err != nil {
			return nil, err
		}
	}

	return &OracleCache{mem, capacity}, nil
}

func (o OracleCache) Clear() {
	o.setSize(0)
}

func (o OracleCache) PutPrice(ticker string, price uint64) bool {
	hash := xxTicker(ticker)
	priceBytes := uint64ToBytes(price)
	first, last := o.findIndex(hash)
	if first == last {
		o.storeAtIndex(first, uint64ToBytes(hash), priceBytes)
		return true
	} else {
		if o.Len() == o.capacity {
			return false
		} else {
			o.setSize(o.Len() + 1)
			o.shiftRight(first)
			o.storeAtIndex(first, uint64ToBytes(hash), priceBytes)
			return true
		}
	}
}

func (o OracleCache) GetPrice(ticker string) *uint64 {
	hash := xxTicker(ticker)
	first, last := o.findIndex(hash)
	if first == last {
		price := bytesToUint64(o.getByIndex(first)[TickerSize:])
		return &price
	} else {
		return nil
	}
}

func (o OracleCache) Len() uint32 {
	bs := make([]byte, 4)
	o.mem.ReadAt(bs, 0)
	return binary.LittleEndian.Uint32(bs)
}

func (o OracleCache) Close() (err error) {
	return o.mem.Close()
}

func (o OracleCache) ToString() string {
	buff := "["
	l := o.Len()
	for i := 0; i < int(l); i++ {
		value := o.getByIndex(uint32(i))
		ticker := bytesToUint64(value[0 : TickerSize+1])
		price := bytesToUint64(value[TickerSize:])
		buff += fmt.Sprintf("%d -> %d, ", ticker, price)
	}
	buff += "]"
	return buff
}

func (o OracleCache) findIndex(ticker uint64) (uint32, uint32) {
	len := o.Len()
	if len == 0 {
		return 0, 1
	}

	first := uint32(0)
	last := len
	for {
		middle := (first + last) / 2
		middleTicker := bytesToUint64(o.getByIndex(middle)[0:TickerSize])
		if ticker == middleTicker {
			return middle, middle
		} else if ticker < middleTicker {
			last = middle
		} else {
			first = middle + 1
		}

		if first >= last {
			last += 1
			break
		}
	}

	return first, last
}

func offset(index uint32) uint32 {
	return ItemSize*index + LenSize
}

func (o OracleCache) storeAtIndex(index uint32, ticker []byte, price []byte) {
	offset := offset(index)
	o.mem.WriteAt(ticker, int64(offset))
	o.mem.WriteAt(price, int64(offset+TickerSize))
}

func (o OracleCache) getByIndex(index uint32) []byte {
	offset := offset(index)
	return o.mem.Slice(int64(offset), int64(ItemSize+offset))
}

func (o OracleCache) setSize(size uint32) {
	bs := make([]byte, 4)
	binary.LittleEndian.PutUint32(bs, size)
	o.mem.WriteAt(bs, 0)
}

func (o OracleCache) shiftRight(index uint32) {
	startOffset := offset(index)
	endOffset := offset(o.Len() - 1)
	bs := make([]byte, endOffset-startOffset)
	o.mem.ReadAt(bs, int64(startOffset))
	o.mem.WriteAt(bs, int64(offset(index+1)))
}

func xxTicker(ticker string) uint64 {
	return xxhash.Sum64String(strings.ToLower(ticker))
}

func uint64ToBytes(int uint64) []byte {
	bs := make([]byte, 8)
	binary.LittleEndian.PutUint64(bs, int)
	return bs
}

func bytesToUint64(price []byte) uint64 {
	return binary.LittleEndian.Uint64(price)
}
