package _go

import (
	"fmt"
	"testing"
)

func TestOracleCache(t *testing.T) {
	cache, err := CreateOracleCache("/test", 100)
	if err != nil {
		t.Errorf("Failed to create cache")
	}
	defer cache.Close()
	cache.Clear()

	cache.PutPrice("BTCUSD", 8000)
	cache.PutPrice("USDRUB", 70)

	if *cache.GetPrice("USDRUB") != 70 {
		t.Errorf("Invalid cache data")
	}

	if *cache.GetPrice("BTCuSD") != 8000 {
		t.Errorf("Invalid cache data")
	}

	if cache.GetPrice("CuSD") != nil {
		t.Errorf("Invalid cache data")
	}

	cache.PutPrice("USDRUB", 80)
	if *cache.GetPrice("USDRUB") != 80 {
		t.Errorf("Invalid cache data")
	}
}

func TestCreateOracleCapacity(t *testing.T) {
	cache, err := CreateOracleCache("/test_1", 100)
	if err != nil {
		t.Errorf("Failed to create cache")
	}
	defer cache.Close()
	cache.Clear()

	for i := 0; i < 100; i++ {
		if !cache.PutPrice(fmt.Sprintf("T:%d", i), uint64(i)) {
			t.Errorf("Failed to put oracle value")
		}
	}

	if cache.Len() != 100 {
		t.Errorf("Invalid cache len")
	}

	if cache.PutPrice("T:101", 101) {
		t.Errorf("Cache overflow")
	}

	for i := 0; i < 100; i++ {
		if *cache.GetPrice(fmt.Sprintf("T:%d", i)) != uint64(i) {
			t.Errorf("Invalid cache value")
		}
	}
}
