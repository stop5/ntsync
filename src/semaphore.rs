use std::{
    io,
    os::fd::AsRawFd as _,
};

use crate::{
    Error,
    EventSources,
    Fd,
    NTSYNC_MAGIC,
    NTSyncObjects,
    NtSync,
    Result,
    Sealed,
    cold_path,
    raw,
};
use derive_new::new;
use log::*;
use nix::{
    errno::Errno,
    ioctl_read,
    ioctl_readwrite,
    ioctl_write_ptr,
    libc,
};


#[repr(C)]
#[derive(Debug, new, Default)]
#[new(visibility = "pub(crate)")]
/// [SemaphoreStatus] is the Status of the Semaphore at the time the [read](Semaphore::read) method was called.
pub struct SemaphoreStatus {
    #[new(value = "max")]
    /// count is the amount that can be allocated.
    ///
    /// it is changed with the [release](Semaphore::release) method and waiting on the Semaphore with [wait_any](NtSync::wait_any) or [wait_all](NtSync::wait_all).
    pub count: u32,
    max: u32,
}

impl SemaphoreStatus {
    /// returns the maximum of allocatable resources.
    pub fn max(&self) -> u32 {
        self.max
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
/// An Semaphore. When the counter reaches 0 Threads will wait until one thread release an specific amount of resources.
/// <div class="warning">Do not release Resources when the ressources are allocated. This can lead to Reduced Perfomance when they are released.</div>
pub struct Semaphore {
    pub(crate) id: Fd,
}
unsafe impl Send for Semaphore {}
unsafe impl Sync for Semaphore {}

impl From<Semaphore> for EventSources {
    fn from(val: Semaphore) -> Self {
        EventSources::Semaphore(val)
    }
}


impl Semaphore {
    /// After the work is done increment the semaphore with this count, so that `amount` threads are woken up.
    /// If an Error was returned the semaphore is NOT changed.
    /// It returns the previous count on return.
    pub fn release(&self, mut amount: u32) -> Result<u32> {
        match unsafe { ntsync_sem_release(self.id, raw!(mut amount: u32)) } {
            Ok(_) => Ok(amount),
            Err(errno) => {
                cold_path();
                match errno {
                    Errno::EOVERFLOW => Err(Error::SemaphoreOverflow),
                    Errno::EBADF => Err(Error::AlreadyClosed),
                    other => {
                        cold_path();
                        Err(Error::Unknown(other as i32))
                    },
                }
            },
        }
    }
}

impl Sealed for Semaphore {}

impl NTSyncObjects for Semaphore {
    type Status = SemaphoreStatus;

    /// deletes the event from the program.
    /// All instances of this event are now invalid
    fn delete(self) -> Result<()> {
        if unsafe { libc::close(self.id) } == -1 {
            cold_path();
            return match Errno::last() {
                Errno::EBADF => {
                    trace!(target: "ntsync", handle=self.id; "tried to double close an Semaphore");
                    Err(Error::AlreadyClosed)
                },
                Errno::EINTR => {
                    trace!(target: "ntsync", handle=self.id; "While closing the Semaphore an interrupt occured");
                    Err(Error::Interrupt)
                },
                Errno::EIO => {
                    trace!(target: "ntsync", handle=self.id; "While closing the Semaphore an IOError occured");
                    Err(Error::IOError(io::Error::from_raw_os_error(Errno::EIO as i32)))
                },
                errno => {
                    cold_path();
                    trace!(target: "ntsync", handle=self.id; "Unexpected error while closing the semaphore: {errno}");
                    Err(Error::Unknown(errno as i32))
                },
            };
        }
        Ok(())
    }

    #[allow(unused)]
    /// Queries the kernel about the current status of the semaphore
    fn read(&self) -> Result<SemaphoreStatus> {
        let mut args = SemaphoreStatus::default();
        match unsafe { ntsync_sem_read(self.id, raw!(mut args: SemaphoreStatus)) } {
            Ok(_) => Ok(args),
            Err(Errno::EBADF) => {
                cold_path();
                Err(Error::AlreadyClosed)
            },
            Err(errno) => {
                cold_path();
                Err(Error::Unknown(errno as i32))
            },
        }
    }
}


impl NtSync {
    /// creates a new Semaphore. it is always initalized with an Maximum between 1 and [u32::MAX] and an count that is the same as the maximum.
    pub fn new_semaphore(&self, maximum: u32) -> Result<Semaphore> {
        let args = SemaphoreStatus::new(maximum.clamp(1, u32::MAX));
        match unsafe { ntsync_create_sem(self.inner.handle.as_raw_fd(), raw!(const args: SemaphoreStatus)) } {
            Ok(fd) => {
                Ok(Semaphore {
                    id: fd,
                })
            },
            Err(errno) => {
                trace!(target: "ntsync",  handle=self.inner.handle.as_raw_fd(), returncode=errno as i32 ;"Failed to create semaphore");
                match errno {
                    Errno::EINVAL => Err(Error::InvalidValue),
                    other => Err(Error::Unknown(other as i32)),
                }
            },
        }
    }
}

//#define NTSYNC_IOC_CREATE_SEM           _IOW ('N', 0x80, struct ntsync_sem_args)
ioctl_write_ptr!(ntsync_create_sem, NTSYNC_MAGIC, 0x80, SemaphoreStatus);
//#define NTSYNC_IOC_SEM_READ             _IOR ('N', 0x8b, struct ntsync_sem_args)
ioctl_read!(ntsync_sem_read, NTSYNC_MAGIC, 0x8B, SemaphoreStatus);
//#define NTSYNC_IOC_SEM_RELEASE          _IOWR('N', 0x81, __u32)
ioctl_readwrite!(ntsync_sem_release, NTSYNC_MAGIC, 0x81, u32);
