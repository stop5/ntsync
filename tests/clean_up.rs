use std::time::{
    Duration,
    SystemTime,
};

use log::*;
use ntsync::{
    Error,
    NTSyncObjects,
    NtSync,
    NtSyncFlags,
    OwnerId,
};
use rstest::rstest;
use test_log::test;

mod fixtures;
use fixtures::*;

macro_rules! test_op {
    ($object_op:expr, $op_name:literal) => {
        match $object_op {
            Err(Error::AlreadyClosed) => {},
            Err(other) => {
                error!("{} returned {:?} not {:?}", $op_name, other, Error::AlreadyClosed);
                return Err(other);
            },
            Ok(_) => {
                error!("{} returned Ok not {:?}", $op_name, Error::InvalidValue);
                return Err(Error::InvalidValue);
            },
        }
    };
}

fn common_tests<T: NTSyncObjects>(object: T, instance: NtSync) -> Result<(), Error> {
    let result =
        instance.wait_any(hash!(object.into()), Some(SystemTime::now() + Duration::from_millis(200)), Some(OwnerId::random()), NtSyncFlags::default(), None);
    match result {
        Ok(_) => return Err(Error::InvalidValue),
        Err(Error::InvalidValue) => {},
        Err(error) => {
            info!("got unexpected error on after trying to wait on object: {error}");
            return Err(error);
        },
    }
    test_op!(object.read(), "object.read");
    test_op!(object.delete(), "object.delete");
    Ok(())
}

#[test(rstest)]
fn delete_event(instance: NtSync) -> Result<(), Error> {
    let object = instance.new_event(true, false)?;
    if let Err(error) = object.delete() {
        error!("Failed to delete event: {error}");
        return Err(error);
    }
    common_tests(object, instance)?;
    test_op!(object.pulse(), "object.pulse");
    test_op!(object.signal(), "object.signal");
    test_op!(object.reset(), "object.reset");
    Ok(())
}

#[test(rstest)]
fn delete_mutex(instance: NtSync) -> Result<(), Error> {
    let object = instance.new_mutex()?;
    if let Err(error) = object.delete() {
        error!("Failed to delete event: {error}");
        return Err(error);
    }
    common_tests(object, instance)?;
    test_op!(object.kill(OwnerId::random()), "object.kill");
    test_op!(object.unlock(OwnerId::random()), "object.unlock");
    Ok(())
}

#[test(rstest)]
fn delete_semaphore(instance: NtSync) -> Result<(), Error> {
    let object = instance.new_semaphore(1)?;
    if let Err(error) = object.delete() {
        error!("Failed to delete event: {error}");
        return Err(error);
    }
    common_tests(object, instance)?;
    Ok(())
}
