use core::time::Duration;
use std::time::SystemTime;
/// The Unix epoch is defined January 1, 1970 00:00:00 UTC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub struct UnixTime(u64, u32); //secs and nanos

impl UnixTime {
    /// The current time, as a `UnixTime`
    pub fn now() -> UnixTime {
        Self::since_unix_epoch(
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap(), // Safe: this code did not exist before 1970.
        )
    }

    /// Convert a `Duration` since the start of 1970 to a `UnixTime`
    ///
    /// The `duration` must be relative to the Unix epoch.
    pub fn since_unix_epoch(duration: Duration) -> Self {
        Self(duration.as_secs(), duration.subsec_nanos())
    }

    /// Number of seconds since the Unix epoch
    pub fn as_secs(&self) -> u64 {
        self.0
    }

    /// Number of seconds since the Unix epoch
    pub fn as_nanos(&self) -> u32 {
        self.1
    }
}

fn main() {
    let before = UnixTime::now();
    dbg!(before);
    dbg!(before.as_secs());
    dbg!(before.as_nanos());
    let after = UnixTime::now();
    dbg!(after);
    assert_ne!(before, after); // NOT equal
}
