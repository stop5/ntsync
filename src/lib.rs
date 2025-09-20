#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(clippy::panic)]
#![forbid(clippy::unimplemented)]
use bitflags::bitflags;
use derive_new::new;
use log::*;
use std::{
    fmt::Display,
    fs::{
        File,
        exists,
    },
    os::fd::RawFd,
    result,
    sync::Arc,
};

mod error;
mod event;
#[cfg(feature = "unstable_mutex")]
#[cfg_attr(docsrs, doc(cfg(feature = "unstable_mutex")))]
mod mutex;
#[cfg(feature = "semaphore")]
#[cfg_attr(docsrs, doc(cfg(feature = "semaphore")))]
mod semaphore;
mod wait;

pub use crate::error::Error;

#[cfg(feature = "semaphore")]
#[cfg_attr(docsrs, doc(cfg(feature = "semaphore")))]
pub use crate::semaphore::Semaphore;
pub use event::{
    Event,
    EventStatus,
};
#[cfg(feature = "unstable_mutex")]
#[cfg_attr(docsrs, doc(cfg(feature = "unstable_mutex")))]
pub use mutex::Mutex;

const DEVICE: &str = "/dev/ntsync";

type Fd = RawFd;

pub(crate) type AlertDescriptor = u32;

// Wrapper around my error Type for Results
pub(crate) type Result<T> = result::Result<T, Error>;

macro_rules! errno_match {
    () => {{
        // TODO: replace with match when inline_const_pat is stable
        let __errno = unsafe { *::libc::__errno_location() };
        trace!("Error number: {__errno}");
        if __errno != 0 {
            return if __errno == ::libc::EINVAL {
                Err(crate::Error::InvalidValue)
            } else if __errno == ::libc::EPERM {
                return Err(crate::Error::PermissionDenied);
            } else if __errno == ::libc::EOVERFLOW {
                return Err(crate::Error::SemaphoreOverflow);
            } else if __errno == ::libc::EINTR {
                return Err(crate::Error::Interrupt);
            } else if __errno == ::libc::EOWNERDEAD {
                return Err(crate::Error::OwnerDead);
            } else if __errno == ::libc::ETIMEDOUT {
                return Err(crate::Error::Timeout);
            } else {
                return Err(crate::Error::Unknown(__errno));
            };
        }
    }};
}

macro_rules! raw {
    (mut $var:ident : $type:ty) => {
        &mut $var as *mut $type
    };
    (const $var:ident : $type:ty) => {
        &$var as *const $type
    };
}

pub(crate) use errno_match;
pub(crate) use raw;

bitflags! {
    #[derive(Debug, Default)]
    pub struct NtSyncFlags: u32 {
        const WaitRealtime = 0x1;
    }
}

#[repr(transparent)]
#[derive(Debug, new, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Default)]
pub struct OwnerId(u32);

#[cfg(feature = "random")]
impl OwnerId {
    pub fn random() -> Self {
        OwnerId(rand::random::<u32>().clamp(1, u32::MAX))
    }
}

impl Display for OwnerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug)]
struct NtSyncInner {
    handle: File,
}

#[derive(Debug)]
pub struct NtSync {
    inner: Arc<NtSyncInner>,
}

/// NtSync is an
impl NtSync {
    pub fn new() -> crate::Result<Self> {
        match exists(DEVICE) {
            Ok(true) => {},
            Ok(false) => return Err(Error::NotExist),
            Err(error) => return Err(Error::IOError(error)),
        }
        match File::open(DEVICE) {
            Ok(file) => {
                Ok(NtSync {
                    inner: Arc::new(NtSyncInner {
                        handle: file,
                    }),
                })
            },
            Err(error) => {
                trace!("Failed to open ntsync device: {error}");
                Err(Error::IOError(error))
            },
        }
    }
}

unsafe impl Send for NtSync {}

impl Clone for NtSync {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}


#[derive(Debug, Hash, PartialEq, Eq)]
pub enum EventSources {
    #[cfg(feature = "unstable_mutex")]
    #[cfg_attr(docsrs, doc(cfg(feature = "unstable_mutex")))]
    Mutex(mutex::Mutex),

    #[cfg(feature = "semaphore")]
    #[cfg_attr(docsrs, doc(cfg(feature = "semaphore")))]
    Semaphore(semaphore::Semaphore),
    Event(event::Event),
}
