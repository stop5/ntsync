use std::time::{
    Duration,
    SystemTime,
};

use log::*;
use ntsync::{
    Error,
    NtSync,
    NtSyncFlags,
    OwnerId,
};
use rstest::rstest;
use test_log::test;

mod fixtures;
use fixtures::*;

#[test(rstest)]
fn test_multiple_instances(instance1: NtSync, instance2: NtSync) -> Result<(), Error> {
    let mutex1 = instance1.new_mutex()?;
    let mutex2 = instance2.new_mutex()?;

    let result = instance1.wait_all(
        hash!(mutex1.into(), mutex2.into()),
        Some(SystemTime::now() + Duration::from_millis(200)),
        Some(OwnerId::random()),
        NtSyncFlags::default(),
        None,
    );
    match result {
        Ok(_) => {
            error!("Failed to correctly wait on objects of different instances.");
            return Err(Error::InvalidValue);
        },
        Err(Error::InvalidValue) => {},
        Err(other) => {
            error!("Failed to wait on objects from different instances: {other:?}");
            return Err(other);
        },
    }
    Ok(())
}
