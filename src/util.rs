use std::time::Duration;

pub trait DurationExt {
	fn as_msecs(&self) -> u64;
}

impl DurationExt for Duration {
	fn as_msecs(&self) -> u64 {
		self.as_secs() * 1_000 + (self.subsec_nanos() / 1_000) as u64
	}
}
