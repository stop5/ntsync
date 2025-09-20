use core::result;
use derive_new::new;
use ioctls::ioctl;
use log::*;
use std::{
    collections::HashSet,
    fmt::Display,
};

use crate::{
    Event,
    EventSources,
    NtSyncFlags,
    error::Error,
};

// Wrapper around my error Type for Results
pub(crate) type Result<T> = result::Result<T, Error>;
pub(crate) type AlertDescriptor = u32;

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

#[repr(C)]
#[derive(Debug, new, Default)]
pub struct SemaphoreArgs {
    #[new(value = "0")]
    pub count: u32,
    pub max: u32,
}

#[cfg(feature = "unstable_mutex")]
#[repr(C)]
#[derive(Debug, new, Default)]
pub struct MutexArgs {
    owner: OwnerId,
    #[new(value = "0")]
    count: libc::__u32,
}

#[repr(C)]
#[derive(Debug, new, Default)]
pub struct EventStatus {
    signaled: libc::__u32,
    manual: libc::__u32,
}

impl EventStatus {
    pub fn manual_signal(&self) -> bool {
        self.manual == 1
    }

    pub fn auto_signal(&self) -> bool {
        self.signaled == 1
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct WaitArgs {
    timeout: u64,
    objs: *const u64,
    count: u32,
    owner: u32,
    index: u32,
    alert: AlertDescriptor,
    flags: u32,
    pad: u32,
}

impl WaitArgs {
    pub(crate) fn new(timeout: u64, eventssources: HashSet<EventSources>, alert: Option<Event>, owner: Option<OwnerId>, flags: NtSyncFlags) -> Result<Self> {
        let mut ids = Vec::new();
        let alertid = alert
            .unwrap_or(Event {
                id: 0,
            })
            .id;
        for source in eventssources {
            match source {
                EventSources::Event(event) => {
                    if alertid != 0 && alertid == event.id {
                        return Err(Error::DuplicateEvent);
                    }
                    ids.push(event.id as u64)
                },
                EventSources::Semaphore(semaphore) => ids.push(semaphore.id as u64),

                #[cfg(feature = "unstable_mutex")]
                EventSources::Mutex(mutex) => {
                    if owner.is_none_or(|val| val.0 == 0) {
                        error!("Invalid Owner. Owner must be an non Zero value");
                        return Err(Error::InvalidValue);
                    }
                    ids.push(mutex.id as u64);
                },
            }
        }
        Ok(Self {
            timeout,
            objs: ids.as_ptr(),
            count: ids.len() as u32,
            owner: owner.unwrap_or_default().0,
            index: 0,
            alert: 0,
            flags: 0,
            pad: 0,
        })
    }
}

//#define NTSYNC_IOC_CREATE_SEM           _IOW ('N', 0x80, struct ntsync_sem_args)
ioctl!(write ntsync_create_sem with b'N', 0x80; SemaphoreArgs);
//#define NTSYNC_IOC_SEM_READ             _IOR ('N', 0x8b, struct ntsync_sem_args)
ioctl!(read ntsync_sem_read with b'N', 0x8b; SemaphoreArgs);
//#define NTSYNC_IOC_SEM_RELEASE          _IOWR('N', 0x81, __u32)
ioctl!(readwrite ntsync_sem_release with b'N', 0x81; u32);

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

//#define NTSYNC_IOC_CREATE_EVENT         _IOW ('N', 0x87, struct ntsync_event_args)
ioctl!(write ntsync_create_event with b'N', 0x87; EventStatus);
//#define NTSYNC_IOC_EVENT_SET            _IOR ('N', 0x88, __u32)
ioctl!(read ntsync_event_set with b'N', 0x88; u32);
//#define NTSYNC_IOC_EVENT_RESET          _IOR ('N', 0x89, __u32)
ioctl!(read ntsync_event_reset with b'N', 0x89; u32);
//#define NTSYNC_IOC_EVENT_PULSE          _IOR ('N', 0x8a, __u32)
ioctl!(read ntsync_event_pulse with b'N', 0x8a; u32);
//#define NTSYNC_IOC_EVENT_READ           _IOR ('N', 0x8d, struct ntsync_event_args)
ioctl!(read ntsync_event_read with b'N', 0x8d; EventStatus);

//#define NTSYNC_IOC_WAIT_ANY             _IOWR('N', 0x82, struct ntsync_wait_args)
ioctl!(readwrite ntsync_wait_any with b'N', 0x82; WaitArgs);
//#define NTSYNC_IOC_WAIT_ALL             _IOWR('N', 0x83, struct ntsync_wait_args)
ioctl!(readwrite ntsync_wait_all with b'N', 0x83; WaitArgs);
