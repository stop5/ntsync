use std::os::fd::AsRawFd as _;

use derive_new::new;
use nix::{
    errno::Errno,
    ioctl_read,
    ioctl_write_ptr,
};

use crate::{
    EventSources,
    Fd,
    NTSYNC_MAGIC,
    NtSync,
    raw,
};
use log::*;


#[repr(C)]
#[derive(Debug, new, Default)]
#[new(visibility = "pub(crate)")]
/// Represents the Status of the Event at the moment of the Query.
pub struct EventStatus {
    manual: u32,
    signaled: u32,
}

impl EventStatus {
    /// Returns true if the event is an manual reset event
    pub fn manual_reset(&self) -> bool {
        self.manual == 1
    }

    /// Returns true if the event was automatically triggered.
    pub fn signaled(&self) -> bool {
        self.signaled == 1
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
/// An Event is an Type that can send Signals to other parts of the code.
/// They can be automatically or manually reset.
/// With manually reset events all waiting threads are worken up, but with automatically reset Event only one wakes up and can do the work.
/// <div class="warning">An Automatically reset Event can trigger multiple times in a row and wake a whole lot of threads up</div>
pub struct Event {
    pub(crate) id: Fd,
}

impl Event {
    /// Triggers the event and signals all Threads waiting on it.
    /// The Event has to be reset after callling the function
    /// It returns if the signal was previously triggered
    pub fn signal(&self) -> crate::Result<bool> {
        let mut state: u32 = 0;
        match unsafe { ntsync_event_set(self.id, raw!(mut state: u32)) } {
            Ok(_) => Ok(state != 0),
            Err(errno) => {
                trace!(target: "ntsync", handle=self.id, returncode=errno as i32 ;"Failed to signal event");
                match errno {
                    Errno::EINVAL => Err(crate::Error::InvalidValue),
                    Errno::EPERM => Err(crate::Error::PermissionDenied),
                    Errno::EOVERFLOW => Err(crate::Error::SemaphoreOverflow),
                    Errno::EINTR => Err(crate::Error::Interrupt),
                    Errno::EOWNERDEAD => Err(crate::Error::OwnerDead),
                    Errno::ETIMEDOUT => Err(crate::Error::Timeout),
                    other => Err(crate::Error::Unknown(other as i32)),
                }
            },
        }
    }

    /// [Event::reset] resets manual Events. It does nothing in Automatic Events
    pub fn reset(&self) -> crate::Result<bool> {
        let mut state: u32 = 0;
        match unsafe { ntsync_event_reset(self.id, raw!(mut state: u32)) } {
            Ok(_) => Ok(state != 0),
            Err(errno) => {
                trace!(target: "ntsync", handle=self.id, returncode=errno as i32 ;"Failed to reset event");
                match errno {
                    Errno::EINVAL => Err(crate::Error::InvalidValue),
                    Errno::EPERM => Err(crate::Error::PermissionDenied),
                    Errno::EOVERFLOW => Err(crate::Error::SemaphoreOverflow),
                    Errno::EINTR => Err(crate::Error::Interrupt),
                    Errno::EOWNERDEAD => Err(crate::Error::OwnerDead),
                    Errno::ETIMEDOUT => Err(crate::Error::Timeout),
                    other => Err(crate::Error::Unknown(other as i32)),
                }
            },
        }
    }

    /// Sets and resets the Event in an Atomic Operation.
    /// Simultanous Reads will show the Event as unsignaled
    pub fn pulse(&self) -> crate::Result<bool> {
        let mut state: u32 = 0;
        match unsafe { ntsync_event_pulse(self.id, raw!(mut state: u32)) } {
            Ok(_) => Ok(state != 0),
            Err(errno) => {
                trace!(target: "ntsync", handle=self.id, returncode=errno as i32 ;"Failed to pulse event");
                match errno {
                    Errno::EINVAL => Err(crate::Error::InvalidValue),
                    Errno::EPERM => Err(crate::Error::PermissionDenied),
                    Errno::EOVERFLOW => Err(crate::Error::SemaphoreOverflow),
                    Errno::EINTR => Err(crate::Error::Interrupt),
                    Errno::EOWNERDEAD => Err(crate::Error::OwnerDead),
                    Errno::ETIMEDOUT => Err(crate::Error::Timeout),
                    other => Err(crate::Error::Unknown(other as i32)),
                }
            },
        }
    }

    /// Returns the Status at the moment of the Query.
    pub fn status(&self) -> crate::Result<EventStatus> {
        let mut args = EventStatus::default();
        match unsafe { ntsync_event_read(self.id, raw!(mut args: EventStatus)) } {
            Ok(_) => Ok(args),
            Err(errno) => {
                trace!(target: "ntsync", handle=self.id, returncode=errno as i32 ;"Failed to query event");
                match errno {
                    Errno::EINVAL => Err(crate::Error::InvalidValue),
                    Errno::EPERM => Err(crate::Error::PermissionDenied),
                    Errno::EOVERFLOW => Err(crate::Error::SemaphoreOverflow),
                    Errno::EINTR => Err(crate::Error::Interrupt),
                    Errno::EOWNERDEAD => Err(crate::Error::OwnerDead),
                    Errno::ETIMEDOUT => Err(crate::Error::Timeout),
                    other => Err(crate::Error::Unknown(other as i32)),
                }
            },
        }
    }
}

unsafe impl Send for Event {}

impl From<Event> for EventSources {
    fn from(val: Event) -> Self {
        EventSources::Event(val)
    }
}

impl NtSync {
    /// Creates a new Event.
    /// if signaled is true the threads begin the work as soo they are waiting.
    /// when manual is true, the event has to be reset manually.
    /// if manual is false after the first thread successful waits on it, the signaled status is set to false.
    pub fn new_event(&self, signaled: bool, manual: bool) -> crate::Result<Event> {
        let args = EventStatus::new(signaled as u32, manual as u32);
        match unsafe { ntsync_create_event(self.inner.handle.as_raw_fd(), raw!(const args: EventStatus)) } {
            Ok(fd) => {
                Ok(Event {
                    id: fd,
                })
            },
            Err(errno) => {
                trace!(target: "ntsync", handle=self.inner.handle.as_raw_fd(), returncode=errno as i32 ;"Failed to create event");
                match errno {
                    Errno::EINVAL => Err(crate::Error::InvalidValue),
                    Errno::EPERM => Err(crate::Error::PermissionDenied),
                    Errno::EOVERFLOW => Err(crate::Error::SemaphoreOverflow),
                    Errno::EINTR => Err(crate::Error::Interrupt),
                    Errno::EOWNERDEAD => Err(crate::Error::OwnerDead),
                    Errno::ETIMEDOUT => Err(crate::Error::Timeout),
                    other => Err(crate::Error::Unknown(other as i32)),
                }
            },
        }
    }
}

//#define NTSYNC_IOC_CREATE_EVENT         _IOW ('N', 0x87, struct ntsync_event_args)
ioctl_write_ptr!(ntsync_create_event, NTSYNC_MAGIC, 0x87, EventStatus);
//#define NTSYNC_IOC_EVENT_SET            _IOR ('N', 0x88, __u32)
ioctl_read!(ntsync_event_set, NTSYNC_MAGIC, 0x88, u32);
//#define NTSYNC_IOC_EVENT_RESET          _IOR ('N', 0x89, __u32)
ioctl_read!(ntsync_event_reset, NTSYNC_MAGIC, 0x89, u32);
//#define NTSYNC_IOC_EVENT_PULSE          _IOR ('N', 0x8a, __u32)
ioctl_read!(ntsync_event_pulse, NTSYNC_MAGIC, 0x8A, u32);
//#define NTSYNC_IOC_EVENT_READ           _IOR ('N', 0x8d, struct ntsync_event_args)
ioctl_read!(ntsync_event_read, NTSYNC_MAGIC, 0x8D, EventStatus);
