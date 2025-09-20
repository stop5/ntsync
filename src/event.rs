use std::os::fd::AsRawFd as _;

use derive_new::new;
use ioctls::ioctl;

use crate::{
    EventSources,
    Fd,
    NtSync,
    errno_match,
    raw,
};
use log::*;


#[repr(C)]
#[derive(Debug, new, Default)]
pub struct EventStatus {
    signaled: u32,
    manual: u32,
}

impl EventStatus {
    pub fn manual_signal(&self) -> bool {
        self.manual == 1
    }

    pub fn auto_signal(&self) -> bool {
        self.signaled == 1
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Event {
    pub(crate) id: Fd,
}

impl Event {
    /// Triggers the event and signals all Threads waiting on it.
    /// The Event has to be reset after callling the function
    /// It returns if the signal was previously triggered
    pub fn signal(&self) -> crate::Result<bool> {
        let mut state: u32 = 0;
        if unsafe { ntsync_event_set(self.id, raw!(mut state: u32)) } == -1 {
            errno_match!();
        }
        Ok(state != 0)
    }

    pub fn reset(&self) -> crate::Result<bool> {
        let mut state: u32 = 0;
        if unsafe { ntsync_event_reset(self.id, raw!(mut state: u32)) } == -1 {
            errno_match!();
        }
        Ok(state != 0)
    }

    pub fn pulse(&self) -> crate::Result<bool> {
        let mut state: u32 = 0;
        if unsafe { ntsync_event_pulse(self.id, raw!(mut state: u32)) } == -1 {
            errno_match!();
        }
        Ok(state != 0)
    }

    pub fn status(&self) -> crate::Result<EventStatus> {
        let mut args = EventStatus::default();
        if unsafe { ntsync_event_read(self.id, raw!(mut args: EventStatus)) } == -1 {
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

impl NtSync {
    pub fn new_event(&self) -> crate::Result<Event> {
        let args = EventStatus::new(0, 0);
        let result = unsafe { ntsync_create_event(self.inner.handle.as_raw_fd(), raw!(const args: EventStatus)) };
        if result < 0 {
            trace!("Failed to create event");
            errno_match!();
        }
        trace!("{args:?}");
        Ok(Event {
            id: result as Fd,
        })
    }
}

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
