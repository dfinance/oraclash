package shm

// Memory is shared memory struct
type Memory struct {
	m *shmi
}

// Create is create shared memory
func Create(name string, size int32) (*Memory, error) {
	m, err := create(name, size)
	if err != nil {
		return nil, err
	}
	return &Memory{m}, nil
}

// Open is open exist shared memory
func Open(name string, size int32) (*Memory, error) {
	m, err := open(name, size)
	if err != nil {
		return nil, err
	}
	return &Memory{m}, nil
}

// Close is close & discard shared memory
func (o *Memory) Close() (err error) {
	if o.m != nil {
		err = o.m.close()
		if err == nil {
			o.m = nil
		}
	}
	return err
}

func (o *Memory) Slice(off int64, size int64) []byte {
	return o.m.memRef(off, size)
}

// ReadAt is read shared memory (offset)
func (o *Memory) ReadAt(p []byte, off int64) int {
	return o.m.readAt(p, off)
}

// WriteAt is write shared memory (offset)
func (o *Memory) WriteAt(p []byte, off int64) int {
	return o.m.writeAt(p, off)
}
