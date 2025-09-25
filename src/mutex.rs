use std::os::fd::AsRawFd as _;

use derive_new::new;
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
#[new(visibility = "pub(crate)")]
/// Mutex Status is the Representation of the Status of the mutex at point of the query
pub struct MutexStatus {
    owner: OwnerId,
    /// This is how deep an thread has relocked the mutex again(mutiple [NtSync::wait_all] or [NtSync::wait_any] calls without unlocking it.)
    #[new(value = "0")]
    count: u32,
}

impl MutexStatus {
    /// The current Owner of the Mutex
    pub fn owner(&self) -> Option<OwnerId> {
        if self.owner.0 == 0 {
            return None;
        }
        Some(self.owner)
    }

    /// how many times the current owner has locked the Mutex.
    pub fn depth(&self) -> Option<u32> {
        if self.count != 0 {
            Some(self.count)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
/// An Mutex similar to [`std::sync::Mutex`], but it can't store Data.
/// On its own it can only be unlocked. The Locking is done in the [NtSync::wait_any] or [NtSync::wait_all] calls.

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
    /// unlocks the Mutex, if its the wrong owner then it fails with [crate::error::Error::PermissionDenied]
    pub fn unlock(&self, owner: OwnerId) -> crate::Result<()> {
        let mut args = MutexStatus::new(owner);
        if unsafe { ntsync_mutex_unlock(self.id, raw!(mut args: MutexStatus)) } == -1 {
            errno_match!()
        }
        Ok(())
    }

    #[allow(unused)]
    /// reads the current status of the Mutex.
    pub fn read(&self) -> crate::Result<MutexStatus> {
        let mut args = MutexStatus::default();
        if unsafe { ntsync_mutex_read(self.id, raw!(mut args: MutexStatus)) } == -1 {
            errno_match!()
        }
        Ok(args)
    }

    /// Forcibly unlocks the Mutex.
    pub fn kill(&self, owner: OwnerId) -> crate::Result<()> {
        let id = owner.0;
        if unsafe { ntsync_mutex_kill(self.id, raw!(const id: u32)) } == -1 {
            error!(target: "ntsync", "Wanted to kill Mutex {}, but failed", self.id);
            errno_match!()
        }
        error!(target: "ntsync", "Mutex {} was killed.", self.id);
        Ok(())
    }
}

impl NtSync {
    /// Creates an unlocked, unowned Mutex.
    pub fn new_mutex(&self) -> crate::Result<Mutex> {
        let args = MutexStatus::default();
        let result = unsafe { ntsync_create_mutex(self.inner.handle.as_raw_fd(), raw!( const args: MutexStatus)) };
        if result < 0 {
            trace!(target: "ntsync", handle=self.inner.handle.as_raw_fd(), returncode=result ;"Failed to create mutex");
            errno_match!();
        }
        trace!("{args:?}");
        Ok(Mutex {
            id: result as Fd,
        })
    }
}

//#define NTSYNC_IOC_CREATE_MUTEX         _IOW ('N', 0x84, struct ntsync_mutex_args)
ioctl!(write ntsync_create_mutex with b'N', 0x84; MutexStatus);
//#define NTSYNC_IOC_MUTEX_UNLOCK         _IOWR('N', 0x85, struct ntsync_mutex_args)
ioctl!(readwrite ntsync_mutex_unlock with b'N', 0x85; MutexStatus);
//#define NTSYNC_IOC_MUTEX_KILL           _IOW ('N', 0x86, __u32)
ioctl!(write ntsync_mutex_kill with b'N', 0x86; u32);
//#define NTSYNC_IOC_MUTEX_READ           _IOR ('N', 0x8c, struct ntsync_mutex_args)
ioctl!(read ntsync_mutex_read with b'N', 0x8c; MutexStatus);
