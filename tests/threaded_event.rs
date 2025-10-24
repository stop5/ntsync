use log::*;
use ntsync::{
    Error,
    NTSyncObjects as _,
    NtSync,
    NtSyncFlags,
    OwnerId,
};
use rstest::rstest;
use std::{
    collections::HashSet,
    thread::{
        Builder,
        JoinHandle,
        sleep,
    },
    time::{
        Duration,
        SystemTime,
    },
};
use test_log::test;

mod fixtures;
use fixtures::*;

#[test(rstest)]
#[cfg(mutex)]
fn test_event_locking(instance: NtSync) -> Result<(), Error> {
    let event = instance.new_event(false, true)?;
    let thread_data = (instance.clone(), event);
    let _thread: JoinHandle<Result<(), Error>> = match Builder::new().name("lock thread".to_owned()).spawn::<_, Result<(), Error>>(move || {
        let (instance, event) = thread_data;
        trace!("Current Status of the event: {:?}", event.status()?);
        let mut sources = HashSet::new();
        sources.insert(event.into());
        let _resp = instance.wait_all(sources, None, None, NtSyncFlags::empty(), None)?;
        Ok(())
    }) {
        Ok(join) => join,
        Err(error) => panic!("Failed to spawn thread for the test: {error}"),
    };

    Ok(())
}
