use anyhow::Error;
use nix::fcntl::OFlag;
use nix::sys::{
    mman::{mmap, munmap, shm_open, MapFlags, ProtFlags},
    stat::fstat,
    stat::Mode,
};
use nix::unistd::{close, ftruncate};
use std::ffi::c_void;
use std::os::unix::io::RawFd;
use std::ptr;
use std::slice;

pub type ShmResult<T> = Result<T, Error>;

/// Shared memory.
pub struct Shm {
    fd: RawFd,
    mem: *mut c_void,
    size: u32,
}

unsafe impl Send for Shm {}

impl Shm {
    /// Open shared memory file.
    pub fn open(name: &str, size: u32) -> ShmResult<Shm> {
        Shm::new(name, size, OFlag::O_RDWR)
    }

    /// Create shared memory file.
    pub fn create(name: &str, size: u32) -> ShmResult<Shm> {
        Shm::new(name, size, OFlag::O_RDWR | OFlag::O_CREAT)
    }

    /// Open or create shared memory file.
    pub fn open_or_create(name: &str, size: u32) -> ShmResult<Shm> {
        match Shm::open(name, size) {
            Ok(shm) => Ok(shm),
            Err(_) => Shm::create(name, size),
        }
    }

    /// Create a new shared memory file.
    fn new(name: &str, size: u32, flag: OFlag) -> ShmResult<Shm> {
        let name = format!("/{}", name);
        let fd = Shm::shm_open(&name, size, flag)?;
        match Shm::mmap_shm(fd, size) {
            Ok(mem) => Ok(Shm { fd, mem, size }),
            Err(err) => {
                close(fd)?;
                Err(err)
            }
        }
    }

    /// Open shared memory file.
    fn shm_open(name: &str, size: u32, flag: OFlag) -> ShmResult<RawFd> {
        let mode = Mode::S_IRUSR | Mode::S_IWUSR | Mode::S_IRGRP | Mode::S_IWGRP;
        let fd = shm_open(name, flag, mode)?;
        if let Ok(stat) = fstat(fd) {
            if stat.st_size == 0 {
                if let Err(err) = ftruncate(fd, size as i64) {
                    close(fd)?;
                    return Err(err.into());
                }
            }
        }

        Ok(fd)
    }

    fn mmap_shm(fd: RawFd, size: u32) -> ShmResult<*mut c_void> {
        Ok(unsafe {
            mmap(
                ptr::null_mut(),
                size as usize,
                ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
                MapFlags::MAP_SHARED,
                fd,
                0,
            )
        }?)
    }

    /// Returns shared memory reference.
    pub fn memory(&self) -> Memory {
        let mem = unsafe { slice::from_raw_parts_mut(self.mem as *mut u8, self.size as usize) };
        Memory {
            inner: mem,
            size: self.size as usize,
        }
    }
}

/// Shared memory.
pub struct Memory<'a> {
    inner: &'a mut [u8],
    pub size: usize,
}

impl<'a> Memory<'a> {
    /// Returns memory reference.
    pub fn mem_ref(&self) -> &[u8] {
        self.inner
    }

    /// Returns mutable memory reference.
    pub fn mem_ref_mut(&mut self) -> &mut [u8] {
        self.inner
    }
}

impl Drop for Shm {
    fn drop(&mut self) {
        unsafe {
            if let Err(err) = munmap(self.mem, self.size as usize) {
                error!("Failed to munmap:{:?}", err);
            }
            if let Err(err) = close(self.fd) {
                error!("Failed to close shared memory file:{:?}", err);
            }
        }
    }
}
