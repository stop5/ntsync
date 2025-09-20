#![allow(unused)]
use std::{
    collections::HashSet,
    ffi::c_int,
    fs::{
        File,
        exists,
    },
    os::fd::{
        AsRawFd,
        RawFd,
    },
    sync::Arc,
};

use bitflags::bitflags;
use libc::{
    __errno_location as get_errno,
    EINTR,
    EINVAL,
    EOVERFLOW,
    EOWNERDEAD,
    EPERM,
    ETIMEDOUT,
    signal,
};
use log::*;

#[cfg(feature = "unstable_mutex")]
use crate::internal::MutexArgs;
pub use crate::internal::{
    EventStatus,
    OwnerId,
};
use crate::internal::{
    SemaphoreArgs,
    WaitArgs,
};
pub use error::Error;

pub mod error;
mod internal;

const DEVICE: &str = "/dev/ntsync";

type Fd = RawFd;

macro_rules! errno_match {
    () => {{
        // TODO: replace with match when inline_const_pat is stable
        let __errno = unsafe { *get_errno() };
        trace!("Error number: {__errno}");
        if __errno != 0 {
            return if __errno == EINVAL {
                Err(Error::InvalidValue)
            } else if __errno == EPERM {
                return Err(Error::PermissionDenied);
            } else if __errno == EOVERFLOW {
                return Err(Error::SemaphoreOverflow);
            } else if __errno == EINTR {
                return Err(Error::Interrupt);
            } else if __errno == EOWNERDEAD {
                return Err(Error::OwnerDead);
            } else if __errno == ETIMEDOUT {
                return Err(Error::Timeout);
            } else {
                unimplemented!("Unimplemented errno: {__errno}")
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

bitflags! {
    #[derive(Debug, Default)]
    pub struct NtSyncFlags: u32 {
        const WaitRealtime = 0x1;
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
    pub fn new() -> internal::Result<Self> {
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

    /// creates a new Semaphore. it is always initalized with an count of 0 and an Maximum between 1 and u32::MAX.
    pub fn new_semaphore(&self, maximum: u32) -> internal::Result<Semaphore> {
        let args = SemaphoreArgs::new(maximum.clamp(1, u32::MAX));
        let result = unsafe { internal::ntsync_create_sem(self.inner.handle.as_raw_fd(), raw!(const args: SemaphoreArgs)) };
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

    #[cfg(feature = "unstable_mutex")]
    pub fn new_mutex(&self) -> internal::Result<Mutex> {
        let args = MutexArgs::default();
        let result: c_int = unsafe { internal::ntsync_create_mutex(self.inner.handle.as_raw_fd(), raw!( const args: MutexArgs)) };
        if result < 0 {
            trace!("Failed to create mutex");
            errno_match!();
        }
        trace!("{args:?}");
        Ok(Mutex {
            id: result as Fd,
        })
    }

    pub fn new_event(&self) -> internal::Result<Event> {
        let args = EventStatus::new(0, 0);
        let result = unsafe { internal::ntsync_create_event(self.inner.handle.as_raw_fd(), raw!(const args: EventStatus)) };
        if result < 0 {
            trace!("Failed to create event");
            errno_match!();
        }
        trace!("{args:?}");
        Ok(Event {
            id: result as Fd,
        })
    }

    pub fn wait_all(&self, sources: HashSet<EventSources>, timeout: Option<u64>, owner: Option<OwnerId>) -> internal::Result<()> {
        let mut args = WaitArgs::new(timeout.unwrap_or(u64::MAX), sources, None, owner, NtSyncFlags::empty())?;
        if unsafe { internal::ntsync_wait_all(self.inner.handle.as_raw_fd(), raw!(mut args: WaitArgs)) } == -1 {
            errno_match!()
        }
        Ok(())
    }

    pub fn wait_any(&self, sources: HashSet<EventSources>, timeout: Option<u64>, owner: Option<OwnerId>) -> internal::Result<()> {
        let mut args = WaitArgs::new(timeout.unwrap_or(u64::MAX), sources, None, owner, NtSyncFlags::empty())?;
        if unsafe { internal::ntsync_wait_any(self.inner.handle.as_raw_fd(), raw!(mut args: WaitArgs)) } == -1 {
            errno_match!()
        }
        Ok(())
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


#[derive(Debug)]
pub enum EventSources {
    #[cfg(feature = "unstable_mutex")]
    Mutex(Mutex),
    Semaphore(Semaphore),
    Event(Event),
}

#[derive(Debug, Clone, Copy)]
pub struct Semaphore {
    id: Fd,
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
    pub fn release(&self, mut amount: u32) -> internal::Result<u32> {
        if unsafe { internal::ntsync_sem_release(self.id, raw!(mut amount: u32)) } == -1 {
            errno_match!()
        }
        Ok(amount)
    }

    pub fn read(&self) -> internal::Result<SemaphoreArgs> {
        let mut args = SemaphoreArgs::default();
        if unsafe { internal::ntsync_sem_read(self.id, raw!(mut args: SemaphoreArgs)) } == -1 {
            errno_match!()
        }
        Ok(args)
    }
}

#[cfg(feature = "unstable_mutex")]
#[derive(Debug, Clone, Copy)]
pub struct Mutex {
    id: Fd,
}

#[cfg(feature = "unstable_mutex")]
unsafe impl Send for Mutex {}

#[cfg(feature = "unstable_mutex")]
impl Into<EventSources> for Mutex {
    fn into(self) -> EventSources {
        EventSources::Mutex(self)
    }
}

#[cfg(feature = "unstable_mutex")]
impl Mutex {
    pub fn unlock(&self, owner: OwnerId) -> internal::Result<()> {
        let mut args = MutexArgs::new(owner);
        if unsafe { internal::ntsync_mutex_unlock(self.id, raw!(mut args: MutexArgs)) } == -1 {
            errno_match!()
        }
        Ok(())
    }

    pub fn read(&self) -> internal::Result<MutexArgs> {
        let mut args = MutexArgs::default();
        if unsafe { internal::ntsync_mutex_read(self.id, raw!(mut args: MutexArgs)) } == -1 {
            errno_match!()
        }
        Ok(args)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Event {
    id: Fd,
}

impl Event {
    /// Triggers the event and signals all Threads waiting on it.
    /// The Event has to be reset after callling the function
    /// It returns if the signal was previously triggered
    pub fn signal(&self) -> internal::Result<bool> {
        let mut state: u32 = 0;
        if unsafe { internal::ntsync_event_set(self.id, raw!(mut state: u32)) } == -1 {
            errno_match!();
        }
        Ok(state != 0)
    }

    pub fn reset(&self) -> internal::Result<bool> {
        let mut state: u32 = 0;
        if unsafe { internal::ntsync_event_reset(self.id, raw!(mut state: u32)) } == -1 {
            errno_match!();
        }
        Ok(state != 0)
    }

    pub fn pulse(&self) -> internal::Result<bool> {
        let mut state: u32 = 0;
        if unsafe { internal::ntsync_event_pulse(self.id, raw!(mut state: u32)) } == -1 {
            errno_match!();
        }
        Ok(state != 0)
    }

    pub fn status(&self) -> internal::Result<EventStatus> {
        let mut args = EventStatus::default();
        if unsafe { internal::ntsync_event_read(self.id as c_int, raw!(mut args: EventStatus)) } == -1 {
            errno_match!()
        }
        trace!("returned args: {args:?}");
        Ok(args)
    }
}

unsafe impl Send for Event {}

impl From<Event> for EventSources {
    fn from(val: Event) -> Self {
        EventSources::Event(val)
    }
}
