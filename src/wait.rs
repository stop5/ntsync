use derive_new::new;
use ioctls::ioctl;
#[allow(unused_imports)]
use log::*;
use std::{
    collections::HashSet,
    os::fd::AsRawFd as _,
    time::{
        SystemTime,
        UNIX_EPOCH,
    },
};

#[allow(unused)]
use crate::{
    AlertDescriptor,
    Error,
    Event,
    EventSources,
    NtSync,
    NtSyncFlags,
    OwnerId,
    errno_match,
    raw,
};

#[repr(C)]
#[derive(Debug, new)]
struct WaitArgs {
    timeout: u64,
    objs: u64,
    count: u32,
    index: u32,
    flags: u32,
    owner: u32,
    alert: u32,
    #[new(value = "0")]
    pad: u32,
}

#[derive(Debug, Clone)]
pub struct WaitAllStatus {
    /// if true the Alert stopped the wait
    pub alerted: bool,
    /// The objects as they were given, so the aquired resources can be freed.
    pub objects: Vec<EventSources>,
}

#[derive(Debug, Clone)]
pub struct WaitAnyStatus {
    /// if true the Alert stopped the wait
    pub alerted: bool,
    /// The objects in the order they were processed
    pub objects: Vec<EventSources>,
    /// the number of the event that stopped the wait. 0 if the Alert stopped it.
    pub index: u32,
}

impl NtSync {
    /// this function waits until all sources are free/triggered.
    /// It is the reason [NtSync::wait_any] also has an [`std::collections::HashSet`] in its signature.
    /// the Kernel Driver reacts with duplicate Values in its event sources or an Event that is both an object and an alert.
    /// this implementation prevents it by making it impossible to reach that state.
    pub fn wait_all(
        &self,
        sources: HashSet<EventSources>,
        timeout: Option<SystemTime>,
        owner: Option<OwnerId>,
        flags: NtSyncFlags,
        alert: Option<Event>,
    ) -> crate::Result<WaitAllStatus> {
        let mut return_sources = Vec::with_capacity(sources.len());
        let mut ids = Vec::new();

        let alertid = alert
            .unwrap_or(Event {
                id: 0,
            })
            .id;
        for source in sources {
            return_sources.push(source);
            match source {
                EventSources::Event(event) => {
                    if alertid != 0 && alertid == event.id {
                        return Err(Error::DuplicateEvent);
                    }
                    ids.push(event.id as u64)
                },
                #[cfg(feature = "semaphore")]
                EventSources::Semaphore(semaphore) => ids.push(semaphore.id as u64),

                #[cfg(feature = "unstable_mutex")]
                EventSources::Mutex(mutex) => {
                    if owner.is_none_or(|val| val.0 == 0) {
                        error!(target: "ntsync", "Invalid Owner. Owner must be an non Zero value");
                        return Err(Error::InvalidValue);
                    }
                    ids.push(mutex.id as u64);
                },
            }
        }
        let mut args = WaitArgs::new(
            timeout.and_then(|st| st.duration_since(UNIX_EPOCH).map(|d| d.as_secs()).ok()).unwrap_or(u64::MAX),
            ids.as_ptr() as u64,
            ids.len() as u32,
            0,
            flags.bits(),
            owner.unwrap_or_default().0,
            alertid as u32,
        );
        if unsafe { ntsync_wait_all(self.inner.handle.as_raw_fd(), raw!(mut args: WaitArgs)) } == -1 {
            trace!(target: "ntsync", handle=self.inner.handle.as_raw_fd(); "Failed to wait on the sources.");
            errno_match!()
        }
        Ok(WaitAllStatus {
            alerted: args.index == args.count,
            objects: return_sources,
        })
    }

    /// this is similar to [NtSync::wait_all], but it will stop waiting once one Source triggers.
    pub fn wait_any(
        &self,
        sources: HashSet<EventSources>,
        timeout: Option<SystemTime>,
        owner: Option<OwnerId>,
        flags: NtSyncFlags,
        alert: Option<Event>,
    ) -> crate::Result<WaitAnyStatus> {
        let mut return_sources = Vec::with_capacity(sources.len());
        let mut ids = Vec::new();

        let alertid = alert
            .unwrap_or(Event {
                id: 0,
            })
            .id;
        for source in sources {
            return_sources.push(source);
            match source {
                EventSources::Event(event) => {
                    if alertid != 0 && alertid == event.id {
                        return Err(Error::DuplicateEvent);
                    }
                    ids.push(event.id as u64)
                },
                #[cfg(feature = "semaphore")]
                EventSources::Semaphore(semaphore) => ids.push(semaphore.id as u64),

                #[cfg(feature = "unstable_mutex")]
                EventSources::Mutex(mutex) => {
                    if owner.is_none_or(|val| val.0 == 0) {
                        error!(target: "ntsync", "Invalid Owner. Owner must be an non Zero value");
                        return Err(Error::InvalidValue);
                    }
                    ids.push(mutex.id as u64);
                },
            }
        }
        let mut args = WaitArgs::new(
            timeout.and_then(|st| st.duration_since(UNIX_EPOCH).map(|d| d.as_secs()).ok()).unwrap_or(u64::MAX),
            ids.as_ptr() as u64,
            ids.len() as u32,
            0,
            flags.bits(),
            owner.unwrap_or_default().0,
            alertid as u32,
        );
        if unsafe { ntsync_wait_any(self.inner.handle.as_raw_fd(), raw!(mut args: WaitArgs)) } == -1 {
            errno_match!()
        }
        Ok(WaitAnyStatus {
            alerted: args.index == args.count,
            objects: return_sources,
            index: if args.index == args.count {
                0
            } else {
                args.count
            },
        })
    }
}

//#define NTSYNC_IOC_WAIT_ANY             _IOWR('N', 0x82, struct ntsync_wait_args)
ioctl!(readwrite ntsync_wait_any with b'N', 0x82; WaitArgs);
//#define NTSYNC_IOC_WAIT_ALL             _IOWR('N', 0x83, struct ntsync_wait_args)
ioctl!(readwrite ntsync_wait_all with b'N', 0x83; WaitArgs);
