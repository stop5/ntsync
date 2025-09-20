use std::os::fd::AsRawFd as _;

use derive_new::new;
#[cfg(feature = "unstable_mutex")]
use ioctls::ioctl;
use log::*;

use crate::{
    EventSources,
    Fd,
    NtSync,
    OwnerId,
    errno_match,
    raw,
};

#[repr(C)]
#[derive(Debug, new, Default)]
pub struct MutexArgs {
    owner: OwnerId,
    #[new(value = "0")]
    count: u32,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Mutex {
    pub(crate) id: Fd,
}

unsafe impl Send for Mutex {}

impl Into<EventSources> for Mutex {
    fn into(self) -> EventSources {
        EventSources::Mutex(self)
    }
}

impl Mutex {
    pub fn unlock(&self, owner: OwnerId) -> crate::Result<()> {
        let mut args = MutexArgs::new(owner);
        if unsafe { ntsync_mutex_unlock(self.id, raw!(mut args: MutexArgs)) } == -1 {
            errno_match!()
        }
        Ok(())
    }

    #[allow(unused)]
    pub fn read(&self) -> crate::Result<MutexArgs> {
        let mut args = MutexArgs::default();
        if unsafe { ntsync_mutex_read(self.id, raw!(mut args: MutexArgs)) } == -1 {
            errno_match!()
        }
        Ok(args)
    }
}

impl NtSync {
    pub fn new_mutex(&self) -> crate::Result<Mutex> {
        let args = MutexArgs::default();
        let result = unsafe { ntsync_create_mutex(self.inner.handle.as_raw_fd(), raw!( const args: MutexArgs)) };
        if result < 0 {
            trace!("Failed to create mutex");
            errno_match!();
        }
        trace!("{args:?}");
        Ok(Mutex {
            id: result as Fd,
        })
    }
}

//#define NTSYNC_IOC_CREATE_MUTEX         _IOW ('N', 0x84, struct ntsync_mutex_args)
#[cfg(feature = "unstable_mutex")]
ioctl!(write ntsync_create_mutex with b'N', 0x84; MutexArgs);
//#define NTSYNC_IOC_MUTEX_UNLOCK         _IOWR('N', 0x85, struct ntsync_mutex_args)
#[cfg(feature = "unstable_mutex")]
ioctl!(readwrite ntsync_mutex_unlock with b'N', 0x85; MutexArgs);
//#define NTSYNC_IOC_MUTEX_KILL           _IOW ('N', 0x86, __u32)
#[cfg(feature = "unstable_mutex")]
ioctl!(write ntsync_mutex_kill with b'N', 0x86; u32);
//#define NTSYNC_IOC_MUTEX_READ           _IOR ('N', 0x8c, struct ntsync_mutex_args)
#[cfg(feature = "unstable_mutex")]
ioctl!(read ntsync_mutex_read with b'N', 0x8c; MutexArgs);
