#![allow(unused_imports)]
use std::{
    os::fd::AsRawFd,
    thread::sleep,
    time::Duration,
};

use log::*;
use ntsync::{
    Error,
    NtSync,
    OwnerId,
};
use rstest::rstest;
use test_log::test;
mod fixtures;
use fixtures::*;

#[test(rstest)]
fn ntsync_event(instance: NtSync) -> Result<(), Error> {
    let event = instance.new_event()?;
    trace!("old value after signal: {}", event.signal()?);
    assert!(event.status()?.manual_signal(), "Event is not signaled");
    trace!("old value after second signal: {}", event.signal()?);
    assert!(event.status()?.manual_signal(), "Event is not signaled");
    trace!("old value after reset: {}", event.reset()?);
    assert!(!event.status()?.manual_signal(), "Event is still signaled");
    trace!("old value after pulse: {}", event.pulse()?);
    Ok(())
}

#[test(rstest)]
#[cfg(feature = "unstable_mutex")]
fn ntsync_mutex(instance: NtSync) -> Result<(), Error> {
    let owner = OwnerId::random();
    let mutex = instance.new_mutex()?;
    assert_eq!(mutex.unlock(owner), Err(Error::PermissionDenied));
    Ok(())
}

#[test(rstest)]
fn ntsync_semaphore(instance: NtSync) -> Result<(), Error> {
    let semaphore = match instance.new_semaphore(3) {
        Ok(event) => event,
        Err(error) => panic!("{}", error),
    };
    assert_eq!(semaphore.release(2), Ok(0), "Wrong Previous value");
    assert_eq!(semaphore.release(2), Err(Error::SemaphoreOverflow), "Semaphore did not correctly overflow");
    let status = semaphore.read()?;
    assert_eq!(status.count, 2, "Wrong value for the count");
    assert_eq!(status.max, 3, "Wrong value for the maximum");

    Ok(())
}
