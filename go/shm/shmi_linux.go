// +build linux,cgo

package shm

/*
#cgo LDFLAGS: -lrt

#include <sys/mman.h>
#include <sys/types.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <stdio.h>
#include <unistd.h>

int _create(const char* name, int size, int flag) {
	mode_t mode = S_IRUSR | S_IWUSR | S_IRGRP | S_IWGRP;

	int fd = shm_open(name, flag, mode);
	if (fd < 0) {
		return -1;
	}

	if (ftruncate(fd, size) != 0) {
		close(fd);
		return -2;
	}
	return fd;
}

int shm_create(const char* name, int size) {
	int flag = O_RDWR | O_CREAT;
	return _create(name, size, flag);
}

int _shm_open(const char* name, int size) {
	int flag = O_RDWR;
	return _create(name, size, flag);
}

void* shm_mmap(int fd, int size) {
	void* p = mmap(
		NULL, size,
		PROT_READ | PROT_WRITE,
		MAP_SHARED, fd, 0);
	if (p == MAP_FAILED) {
		return NULL;
	}
	return p;
}

void shm_close(int fd, void* p, int size) {
	if (p != NULL) {
		munmap(p, size);
	}
	if (fd != 0) {
		close(fd);
	}
}

void shm_delete(const char* name) {
	shm_unlink(name);
}
*/
import "C"

import (
	"fmt"
	"unsafe"
)

type shmi struct {
	name   string
	fd     C.int
	v      unsafe.Pointer
	size   int32
	parent bool
}

// create shared memory. return shmi object.
func create(name string, size int32) (*shmi, error) {
	name = "/" + name

	fd := C.shm_create(C.CString(name), C.int(size))
	if fd < 0 {
		return nil, fmt.Errorf("create:%v", fd)
	}

	v := C.shm_map(fd, C.int(size))
	if v == nil {
		C.shm_close(fd, nil, C.int(size))
		C.shm_delete(C.CString(name))
	}

	return &shmi{name, fd, v, size, true}, nil
}

// open shared memory. return shmi object.
func open(name string, size int32) (*shmi, error) {
	name = "/" + name

	fd := C._shm_open(C.CString(name), C.int(size))
	if fd < 0 {
		return nil, fmt.Errorf("open:%v", fd)
	}

	v := C.shm_map(fd, C.int(size))
	if v == nil {
		C.shm_close(fd, nil, C.int(size))
		C.shm_delete(C.CString(name))
	}

	return &shmi{name, fd, v, size, false}, nil
}

func (o *shmi) close() error {
	if o.v != nil {
		C.shm_close(o.fd, o.v, C.int(o.size))
		o.v = nil
	}
	if o.parent {
		C.shm_delete(C.CString(o.name))
	}
	return nil
}

// read shared memory. return read size.
func (o *shmi) readAt(p []byte, off int64) int {
	if max := int64(o.size) - off; int64(len(p)) > max {
		p = p[:max]
	}
	return copyPtr2Slice(uintptr(o.v), p, off, o.size)
}

// write shared memory. return write size.
func (o *shmi) writeAt(p []byte, off int64) int {
	if max := int64(o.size) - off; int64(len(p)) > max {
		p = p[:max]
	}
	return copySlice2Ptr(p, uintptr(o.v), off, o.size)
}

func (o *shmi) memRef(off int64, size int64) []byte {
	h := reflect.SliceHeader{}
	h.Cap = int(o.size)
	h.Len = int(o.size)
	h.Data = uintptr(o.v)
	bb := *(*[]byte)(unsafe.Pointer(&h))

	return bb[off:size]
}
