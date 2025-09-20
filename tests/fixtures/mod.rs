use ntsync::NtSync;
use rstest::fixture;

#[fixture]
pub fn instance() -> NtSync {
    match NtSync::new() {
        Err(error) => {
            panic!("Failed to open Device: {error}")
        },
        Ok(ntsync) => ntsync,
    }
}
