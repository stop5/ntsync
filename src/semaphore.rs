use std::os::fd::AsRawFd as _;

use crate::{
    EventSources,
    Fd,
    NtSync,
    errno_match,
    raw,
};
use derive_new::new;
use ioctls::ioctl;
use log::*;


#[repr(C)]
#[derive(Debug, new, Default)]
pub struct SemaphoreArgs {
    #[new(value = "0")]
    pub count: u32,
    pub max: u32,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
/// An Semaphore. When the counter reaches 0 Threads will wait until one thread release an specific amount of resources.
/// <div class="warning">Do not release Resources when the ressources are allocated. This can lead to Reduced Perfomance when they are released.</div>
pub struct Semaphore {
    pub(crate) id: Fd,
}
unsafe impl Send for Semaphore {}

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
        if unsafe { ntsync_sem_release(self.id, raw!(mut amount: u32)) } == -1 {
            errno_match!()
        }
        Ok(amount)
    }

    #[allow(unused)]
    /// Queries the kernel about the current status of the semaphore
    pub fn read(&self) -> crate::Result<SemaphoreArgs> {
        let mut args = SemaphoreArgs::default();
        if unsafe { ntsync_sem_read(self.id, raw!(mut args: SemaphoreArgs)) } == -1 {
            errno_match!()
        }
        Ok(args)
    }
}


impl NtSync {
    /// creates a new Semaphore. it is always initalized with an count of 0 and an Maximum between 1 and [u32::MAX].
    pub fn new_semaphore(&self, maximum: u32) -> crate::Result<Semaphore> {
        let args = SemaphoreArgs::new(maximum.clamp(1, u32::MAX));
        let result = unsafe { ntsync_create_sem(self.inner.handle.as_raw_fd(), raw!(const args: SemaphoreArgs)) };
        trace!("result of create_sem: {result}");
        if result < 0 {
            trace!("Failed to create semaphore");
            errno_match!();
        }
        trace!("{args:?}");
        Ok(Semaphore {
            id: result as Fd,
        })
    }
}

//#define NTSYNC_IOC_CREATE_SEM           _IOW ('N', 0x80, struct ntsync_sem_args)
ioctl!(write ntsync_create_sem with b'N', 0x80; SemaphoreArgs);
//#define NTSYNC_IOC_SEM_READ             _IOR ('N', 0x8b, struct ntsync_sem_args)
ioctl!(read ntsync_sem_read with b'N', 0x8b; SemaphoreArgs);
//#define NTSYNC_IOC_SEM_RELEASE          _IOWR('N', 0x81, __u32)
ioctl!(readwrite ntsync_sem_release with b'N', 0x81; u32);
