#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(clippy::panic)]
#![forbid(clippy::unimplemented)]
#![warn(missing_docs)]
#![doc = include_str!("../README.md")]
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
#[cfg(feature = "mutex")]
#[cfg_attr(docsrs, doc(cfg(feature = "mutex")))]
mod mutex;
#[cfg(feature = "semaphore")]
#[cfg_attr(docsrs, doc(cfg(feature = "semaphore")))]
mod semaphore;
mod wait;

pub use crate::error::Error;

#[cfg(feature = "semaphore")]
#[cfg_attr(docsrs, doc(cfg(feature = "semaphore")))]
pub use crate::semaphore::{
    Semaphore,
    SemaphoreStatus,
};
pub use event::{
    Event,
    EventStatus,
};
#[cfg(feature = "mutex")]
#[cfg_attr(docsrs, doc(cfg(feature = "mutex")))]
pub use mutex::{
    Mutex,
    MutexStatus,
};

const DEVICE: &str = "/dev/ntsync";
const NTSYNC_MAGIC: u8 = b'N';

type Fd = RawFd;

// Wrapper around my error Type for Results
pub(crate) type Result<T> = result::Result<T, Error>;

macro_rules! raw {
    (mut $var:ident : $type:ty) => {
        &mut $var as *mut $type
    };
    (const $var:ident : $type:ty) => {
        &$var as *const $type
    };
}

pub(crate) use raw;

bitflags! {
    #[derive(Debug, Default)]
    /// This helps Managing the Flags for waiting on Events.
    pub struct NtSyncFlags: u32 {
        /// This causes the Kernel to use the Realtime Clock instead of the monotonic clock.
        const WaitRealtime = 0x1;
    }
}

#[repr(transparent)]
#[derive(Debug, new, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Default)]
/// An [OwnerId] is just an identifier for an part of the code which needs protections against parallel Access.
///
/// The Kernel Module does not check if it matches something else than an number
pub struct OwnerId(u32);

#[cfg(feature = "random")]
#[cfg_attr(docsrs, doc(cfg(feature = "semaphore")))]
impl OwnerId {
    /// Generates an random Owner
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
#[doc(hidden)]
struct NtSyncInner {
    handle: File,
}

#[derive(Debug)]
/// [NtSync] is an abstration over the Kernel API that is realised via ioctls.
///
/// Each instance is indipendent so using objects from one instance with another is forbidden.
pub struct NtSync {
    inner: Arc<NtSyncInner>,
}

impl NtSync {
    /// Creates an new instance of NtSync
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
                trace!(target: "ntsync","Failed to open ntsync device: {error}");
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


#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
/// An Wrapper around the different Syncronisation Primitives of this crate
///
/// EventSources is an enum, so that the different types can coexist in an [HashSet](std::collections::HashSet), [Vec] or any other type dealing with them,
pub enum EventSources {
    #[cfg(feature = "mutex")]
    #[cfg_attr(docsrs, doc(cfg(feature = "mutex")))]
    /// The Wrapper for [Mutex]
    Mutex(mutex::Mutex),

    #[cfg(feature = "semaphore")]
    #[cfg_attr(docsrs, doc(cfg(feature = "semaphore")))]
    /// The Wrapper for [Semaphore]
    Semaphore(semaphore::Semaphore),
    /// An simple wrapper around [Events](Event)
    Event(event::Event),
}

impl EventSources {
    /// Frees the respective resource
    #[cfg_attr(feature = "mutex", doc = "- [Mutex](crate::mutex::Mutex) are unlocked.")]
    #[cfg_attr(
        feature = "semaphore",
        doc = "- [Semaphore](crate::semaphore::Semaphore) are released with an amount of 1."
    )]
    #[doc = "- [Event](crate::event::Event) are reset."]
    pub fn free(&self, _owner: OwnerId) -> Result<()> {
        match self {
            #[cfg(feature = "mutex")]
            EventSources::Mutex(mutex) => {
                mutex.unlock(_owner)?;
            },
            #[cfg(feature = "semaphore")]
            EventSources::Semaphore(semaphore) => {
                semaphore.release(1)?;
            },
            EventSources::Event(event) => {
                event.reset()?;
            },
        };
        Ok(())
    }
}
