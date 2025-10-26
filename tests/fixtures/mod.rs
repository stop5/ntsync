use ntsync::NtSync;
use rstest::fixture;

#[macro_export]
macro_rules! hash {
    ($($item:expr),*) => {
        {
        let mut set = ::std::collections::HashSet::new();
        $(
            set.insert($item);
        )*
        set
        }
    };
}


#[fixture]
pub fn instance() -> NtSync {
    match NtSync::new() {
        Err(error) => {
            panic!("Failed to open Device: {error}")
        },
        Ok(ntsync) => ntsync,
    }
}

#[fixture]
pub fn instance1() -> NtSync {
    instance()
}

#[fixture]
pub fn instance2() -> NtSync {
    instance()
}
