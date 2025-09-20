#![allow(unused_imports)]
use log::*;
use ntsync::{
    Error,
    NtSync,
    OwnerId,
};
use rstest::rstest;
use std::{
    collections::HashSet,
    thread::{
        Builder,
        JoinHandle,
    },
};
use test_log::test;

mod fixtures;
use fixtures::*;

#[test(rstest)]
#[cfg(feature = "unstable_mutex")]
fn test_mutex_locking(instance: NtSync) -> Result<(), Error> {
    let mutex = instance.new_mutex()?;
    let thread_data = (instance.clone(), mutex, OwnerId::random());
    let owner = OwnerId::random();
    trace!("My owner: {}\nother owner: {}", owner, thread_data.2);
    instance.wait_all(&[mutex.into()], None, Some(owner))?;
    let thread: JoinHandle<Result<(), Error>> = match Builder::new().name("lock thread".to_owned()).spawn::<_, Result<(), Error>>(move || {
        let (instance, mutex, _owner) = thread_data;
        debug!("current owner of the mutex: {:?}", mutex.read());
        let mut sources = HashSet::new();
        sources.insert(mutex.into());
        let resp = instance.wait_all(sources, None, Some(_owner))?;
        Ok(())
    }) {
        Ok(join) => join,
        Err(error) => panic!("Failed to spawn thread for the test: {error}"),
    };
    let mut failed = false;
    match thread.join() {
        Ok(Err(error)) => return Err(error),
        Ok(Ok(())) => {},
        Err(error) => {
            error!("Failed to executed threat correctly: {error:?}");
            failed = true;
        },
    }
    if failed {
        panic!();
    }
    Ok(())
}
