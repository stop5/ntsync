#![allow(unused_imports)]
use log::*;
use ntsync::{
    Error,
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
#[cfg(feature = "mutex")]
fn test_semaphore_locking(instance: NtSync) -> Result<(), Error> {
    let semaphore = instance.new_semaphore(1)?;
    let thread_data = (instance.clone(), semaphore);
    let _thread: JoinHandle<Result<(), Error>> = match Builder::new().name("lock thread".to_owned()).spawn::<_, Result<(), Error>>(move || {
        let (instance, semaphore) = thread_data;
        trace!("Current Status of the semaphore: {:?}", semaphore.read()?);
        let mut sources = HashSet::new();
        sources.insert(semaphore.into());
        let _resp = instance.wait_all(sources, None, None, NtSyncFlags::empty(), None)?;
        Ok(())
    }) {
        Ok(join) => join,
        Err(error) => panic!("Failed to spawn thread for the test: {error}"),
    };

    Ok(())
}
