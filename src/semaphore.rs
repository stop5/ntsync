use std::os::fd::AsRawFd as _;

use crate::{
    EventSources,
    Fd,
    NTSYNC_MAGIC,
    NtSync,
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
    pub fn release(&self, mut amount: u32) -> crate::Result<u32> {
        match unsafe { ntsync_sem_release(self.id, raw!(mut amount: u32)) } {
            Ok(_) => Ok(amount),
            Err(errno) => {
                cold_path();
                match errno {
                    Errno::EOVERFLOW => Err(crate::Error::SemaphoreOverflow),
                    other => {
                        cold_path();
                        Err(crate::Error::Unknown(other as i32))
                    },
                }
            },
        }
    }

    #[allow(unused)]
    /// Queries the kernel about the current status of the semaphore
    pub fn read(&self) -> crate::Result<SemaphoreStatus> {
        let mut args = SemaphoreStatus::default();
        match unsafe { ntsync_sem_read(self.id, raw!(mut args: SemaphoreStatus)) } {
            Ok(_) => Ok(args),
            Err(errno) => {
                cold_path();
                Err(crate::Error::Unknown(errno as i32))
            },
        }
    }
}


impl NtSync {
    /// creates a new Semaphore. it is always initalized with an Maximum between 1 and [u32::MAX] and an count that is the same as the maximum.
    pub fn new_semaphore(&self, maximum: u32) -> crate::Result<Semaphore> {
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
                    Errno::EINVAL => Err(crate::Error::InvalidValue),
                    other => Err(crate::Error::Unknown(other as i32)),
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
