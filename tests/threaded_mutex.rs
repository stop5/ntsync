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
    thread::{
        Builder,
        JoinHandle,
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
fn test_mutex_locking(instance: NtSync) -> Result<(), Error> {
    let mutex = instance.new_mutex()?;
    let thread_data = (instance.clone(), mutex, OwnerId::random());
    let thread: JoinHandle<Result<(), Error>> = match Builder::new().name("lock thread".to_owned()).spawn::<_, Result<(), Error>>(move || {
        let (instance, mutex, owner) = thread_data;
        debug!("current owner of the mutex: {:?}", mutex.read());
        let _resp = instance.wait_all(hash!(mutex.into()), None, Some(owner), NtSyncFlags::empty(), None)?;
        Ok(())
    }) {
        Ok(join) => join,
        Err(error) => panic!("Failed to spawn thread for the test: {error}"),
    };
    match thread.join() {
        Ok(Err(error)) => return Err(error),
        Ok(Ok(())) => {},
        Err(error) => {
            panic!("Failed to executed threat correctly: {error:?}");
        },
    }

    let owner = OwnerId::random();
    trace!("My owner: {} other owner: {}", owner, thread_data.2);
    match instance.wait_all(hash!(mutex.into()), Some(SystemTime::now() + Duration::from_millis(200)), Some(owner), NtSyncFlags::empty(), None) {
        Err(Error::Timeout) => {},
        Err(error) => return Err(error),
        Ok(status) => {
            panic!("this shouldn't happen: {status:?}")
        },
    }
    Ok(())
}
