use log::*;
use ntsync::{
    Error,
    NTSyncObjects as _,
    NtSync,
    NtSyncFlags,
    OwnerId,
};
use rstest::rstest;
use test_log::test;

mod fixtures;
use fixtures::*;

#[test(rstest)]
fn ntsync_event(instance: NtSync) -> Result<(), Error> {
    let event = instance.new_event(false, false)?;
    trace!("old value after signal: {}", event.signal()?);
    trace!("Status: {}", event.status()?.signaled());
    assert!(event.status()?.signaled(), "Event is not signaled");
    trace!("old value after second signal: {}", event.signal()?);
    trace!("Status: {}", event.status()?.signaled());
    assert!(event.status()?.signaled(), "Event is not signaled");
    trace!("old value after reset: {}", event.reset()?);
    trace!("Status: {}", event.status()?.signaled());
    assert!(!event.status()?.signaled(), "Event is still signaled");
    trace!("old value after pulse: {}", event.pulse()?);
    trace!("Status: {}", event.status()?.signaled());
    Ok(())
}

#[test(rstest)]
#[cfg(mutex)]
fn ntsync_mutex(instance: NtSync) -> Result<(), Error> {
    let owner = OwnerId::random();
    let mutex = instance.new_mutex()?;
    assert_eq!(mutex.unlock(owner), Err(Error::PermissionDenied));
    instance.wait_all(hash!(mutex.into()), None, Some(owner), NtSyncFlags::empty(), None)?;
    Ok(())
}

#[test(rstest)]
#[cfg(semaphore)]
fn ntsync_semaphore(instance: NtSync) -> Result<(), Error> {
    let semaphore = match instance.new_semaphore(3) {
        Ok(event) => event,
        Err(error) => panic!("{}", error),
    };
    assert_eq!(semaphore.release(2), Err(Error::SemaphoreOverflow), "Semaphore did not correctly overflow");
    let _ = instance.wait_all(hash!(semaphore.into()), None, None, NtSyncFlags::empty(), None);
    let status = semaphore.read()?;
    assert_eq!(status.count, 2, "Wrong value for the count");
    assert_eq!(status.max(), 3, "Wrong value for the maximum");
    semaphore.release(1)?;
    Ok(())
}
