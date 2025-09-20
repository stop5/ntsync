use std::{
    collections::HashSet,
    os::fd::AsRawFd as _,
};

use ioctls::ioctl;
#[allow(unused_imports)]
use log::*;

use crate::Event;
#[allow(unused)]
use crate::{
    AlertDescriptor,
    Error,
    EventSources,
    NtSync,
    NtSyncFlags,
    OwnerId,
    errno_match,
    raw,
};

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
    pub(crate) fn new(
        timeout: u64,
        eventssources: HashSet<EventSources>,
        alert: Option<Event>,
        owner: Option<OwnerId>,
        flags: NtSyncFlags,
    ) -> crate::Result<Self> {
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
                #[cfg(feature = "semaphore")]
                EventSources::Semaphore(semaphore) => ids.push(semaphore.id as u64),

                #[cfg(feature = "unstable_mutex")]
                EventSources::Mutex(mutex) => {
                    if owner.is_none_or(|val| val.0 == 0) {
                        use crate::Error;

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
            flags: flags.bits(),
            pad: 0,
        })
    }
}

/// The timeout argument of [NtSync::wait_all] and [NtSync::wait_any] is unstable and will be changed to an different Type.
impl NtSync {
    /// this function waits until all sources are free/triggered.
    /// It is the reason [NtSync::wait_any] also has an [`std::collections::HashSet`] in its signature.
    /// the Kernel Driver reacts with duplicate Values in its event sources or an Event that is both an object and an alert.
    /// this implementation prevents it by making it impossible to reach that state.
    pub fn wait_all(&self, sources: HashSet<EventSources>, timeout: Option<u64>, owner: Option<OwnerId>) -> crate::Result<()> {
        let mut args = WaitArgs::new(timeout.unwrap_or(u64::MAX), sources, None, owner, NtSyncFlags::empty())?;
        if unsafe { ntsync_wait_all(self.inner.handle.as_raw_fd(), raw!(mut args: WaitArgs)) } == -1 {
            errno_match!()
        }
        Ok(())
    }

    /// this is similar to [NtSync::wait_all], but it will stop waiting once one Source triggers.
    pub fn wait_any(&self, sources: HashSet<EventSources>, timeout: Option<u64>, owner: Option<OwnerId>) -> crate::Result<()> {
        let mut args = WaitArgs::new(timeout.unwrap_or(u64::MAX), sources, None, owner, NtSyncFlags::empty())?;
        if unsafe { ntsync_wait_any(self.inner.handle.as_raw_fd(), raw!(mut args: WaitArgs)) } == -1 {
            errno_match!()
        }
        Ok(())
    }
}

//#define NTSYNC_IOC_WAIT_ANY             _IOWR('N', 0x82, struct ntsync_wait_args)
ioctl!(readwrite ntsync_wait_any with b'N', 0x82; WaitArgs);
//#define NTSYNC_IOC_WAIT_ALL             _IOWR('N', 0x83, struct ntsync_wait_args)
ioctl!(readwrite ntsync_wait_all with b'N', 0x83; WaitArgs);
