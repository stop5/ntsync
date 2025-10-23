use std::{
    io,
    os::fd::AsRawFd as _,
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

use crate::{
    Error,
    EventSources,
    Fd,
    NTSYNC_MAGIC,
    NtSync,
    OwnerId,
    cold_path,
    raw,
};

#[repr(C)]
#[derive(Debug, new, Default)]
#[new(visibility = "pub(crate)")]
/// Mutex Status is the Representation of the Status of the mutex at point of the query
pub struct MutexStatus {
    owner: OwnerId,
    /// This is how deep an thread has relocked the mutex again(mutiple [wait_any](NtSync::wait_any) or [wait_all](NtSync::wait_all) calls without unlocking it.)
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
/// An Mutex similar to [std::sync::Mutex], but it can't store Data.
///
/// On its own it can only be unlocked. The Locking is done in the [wait_any](NtSync::wait_any) or [wait_all](NtSync::wait_all) calls.
pub struct Mutex {
    pub(crate) id: Fd,
}

unsafe impl Send for Mutex {}
unsafe impl Sync for Mutex {}

impl From<Mutex> for EventSources {
    fn from(value: Mutex) -> EventSources {
        EventSources::Mutex(value)
    }
}

impl Mutex {
    /// unlocks the Mutex, if its the wrong owner then it fails with [PermissionDenied](crate::error::Error::PermissionDenied)
    pub fn unlock(&self, owner: OwnerId) -> crate::Result<()> {
        let mut args = MutexStatus::new(owner);
        match unsafe { ntsync_mutex_unlock(self.id, raw!(mut args: MutexStatus)) } {
            Ok(_) => Ok(()),
            Err(errno) => {
                cold_path();
                match errno {
                    Errno::EINVAL => Err(crate::Error::InvalidValue),
                    Errno::EPERM => Err(crate::Error::PermissionDenied),
                    other => {
                        cold_path();
                        Err(crate::Error::Unknown(other as i32))
                    },
                }
            },
        }
    }

    #[allow(unused)]
    /// reads the current status of the Mutex.
    pub fn read(&self) -> crate::Result<MutexStatus> {
        let mut args = MutexStatus::default();
        match unsafe { ntsync_mutex_read(self.id, raw!(mut args: MutexStatus)) } {
            Ok(_) => Ok(args),
            Err(errno) => {
                cold_path();
                match errno {
                    Errno::EOWNERDEAD => Err(crate::Error::OwnerDead),
                    other => {
                        cold_path();
                        Err(crate::Error::Unknown(other as i32))
                    },
                }
            },
        }
    }

    /// Forcibly unlocks the Mutex.
    pub fn kill(&self, owner: OwnerId) -> crate::Result<()> {
        let id = owner.0;
        match unsafe { ntsync_mutex_kill(self.id, raw!(const id: u32)) } {
            Ok(_) => {
                error!(target: "ntsync", "Mutex {} was killed.", self.id);
                Ok(())
            },
            Err(errno) => {
                cold_path();
                error!(target: "ntsync", "Wanted to kill Mutex {}, but failed", self.id);
                match errno {
                    Errno::EINVAL => Err(crate::Error::InvalidValue),
                    Errno::EPERM => Err(crate::Error::PermissionDenied),
                    other => {
                        cold_path();
                        Err(crate::Error::Unknown(other as i32))
                    },
                }
            },
        }
    }

    /// deletes the Mutex from the program.
    /// All instances of this Mutex are now invalid
    pub fn delete(self) -> crate::Result<()> {
        if unsafe { libc::close(self.id) } == -1 {
            cold_path();
            return match Errno::last() {
                Errno::EBADF => {
                    trace!(target: "ntsync", handle=self.id; "tried to double close an Mutex");
                    Err(Error::DoubleClose)
                },
                Errno::EINTR => {
                    trace!(target: "ntsync", handle=self.id; "While closing the Mutex an interrupt occured");
                    Err(Error::Interrupt)
                },
                Errno::EIO => {
                    trace!(target: "ntsync", handle=self.id; "While closing the Mutex an IOError occured");
                    Err(Error::IOError(io::Error::from_raw_os_error(Errno::EIO as i32)))
                },
                errno => {
                    cold_path();
                    trace!(target: "ntsync", handle=self.id; "Unexpected error while closing the Mutex: {errno}");
                    Err(Error::Unknown(errno as i32))
                },
            };
        }
        Ok(())
    }
}

impl NtSync {
    /// Creates an unlocked, unowned Mutex.
    pub fn new_mutex(&self) -> crate::Result<Mutex> {
        let args = MutexStatus::default();
        match unsafe { ntsync_create_mutex(self.inner.handle.as_raw_fd(), raw!(const args: MutexStatus)) } {
            Ok(fd) => {
                Ok(Mutex {
                    id: fd,
                })
            },
            Err(errno) => {
                cold_path();
                trace!(target: "ntsync", handle=self.inner.handle.as_raw_fd(), returncode=errno as i32 ;"Failed to create Mutex");
                match errno {
                    Errno::EINVAL => Err(crate::Error::InvalidValue),
                    other => {
                        cold_path();
                        Err(crate::Error::Unknown(other as i32))
                    },
                }
            },
        }
    }
}

//#define NTSYNC_IOC_CREATE_MUTEX         _IOW ('N', 0x84, struct ntsync_mutex_args)
ioctl_write_ptr!(ntsync_create_mutex, NTSYNC_MAGIC, 0x84, MutexStatus);
//#define NTSYNC_IOC_MUTEX_UNLOCK         _IOWR('N', 0x85, struct ntsync_mutex_args)
ioctl_readwrite!(ntsync_mutex_unlock, NTSYNC_MAGIC, 0x85, MutexStatus);
//#define NTSYNC_IOC_MUTEX_KILL           _IOW ('N', 0x86, __u32)
ioctl_write_ptr!(ntsync_mutex_kill, NTSYNC_MAGIC, 0x86, u32);
//#define NTSYNC_IOC_MUTEX_READ           _IOR ('N', 0x8c, struct ntsync_mutex_args)
ioctl_read!(ntsync_mutex_read, NTSYNC_MAGIC, 0x8C, MutexStatus);
