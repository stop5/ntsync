use std::{
    fmt::Display,
    io::Error as IOError,
};

#[derive(Debug)]
pub enum Error {
    /// Returned when the /dev/ntsync device does not exists
    NotExist,
    /// Wrapper for the IOError, for example when the device has access problems
    IOError(IOError),
    /// Generic Error returned when the arguments are wrong
    InvalidValue,
    /// returned when an function calls the release method with an higher amount than there is currently used
    SemaphoreOverflow,
    /// The Freeing/killing of the mutex is not permitted with this owner id
    PermissionDenied,
    /// The wait timed out.
    Timeout,
    /// The owner was forcefully stopped.
    OwnerDead,
    /// Process was interrupted by an os signal
    Interrupt,
    /// When an Event is part of the sources and the alert that stops the wait.
    DuplicateEvent,
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::NotExist, Self::NotExist) => true,
            (Self::InvalidValue, Self::InvalidValue) => true,
            (Self::SemaphoreOverflow, Self::SemaphoreOverflow) => true,
            (Self::PermissionDenied, Self::PermissionDenied) => true,
            (Self::Timeout, Self::Timeout) => true,
            (Self::OwnerDead, Self::OwnerDead) => true,
            (Self::Interrupt, Self::Interrupt) => true,
            (..) => false,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NotExist => f.write_str("Device does not Exist"),
            Error::IOError(error) => f.write_fmt(format_args!("IOError: {error}")),
            Error::InvalidValue => f.write_str("Invalid Value for the operation"),
            Error::SemaphoreOverflow => f.write_str("adding the Value to the semaphore exceeds the maximum"),
            Error::PermissionDenied => f.write_str("Cannot Unlock the Mutex. It is owned by another process"),
            Error::Timeout => f.write_str("Waiting timed out"),
            Error::OwnerDead => f.write_str("Owner of the mutex was killed."),
            Error::Interrupt => f.write_str("Interrupt received"),
            Error::DuplicateEvent => f.write_str("An Event is part of the sources and was added as an Alert"),
        }
    }
}
