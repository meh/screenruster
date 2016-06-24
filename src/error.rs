use std::fmt;
use std::error;
use std::io;

#[cfg(feature = "dbus")]
use dbus;

pub type Result<T> = ::std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
	Io(io::Error),
	Locker(Locker),

	#[cfg(feature = "dbus")]
	DBus(dbus::Error),

	Parse,
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum Locker {
	NoDisplay,
	NoVisual,
	NoIM,
	NoIC,
}

impl From<io::Error> for Error {
	fn from(value: io::Error) -> Self {
		Error::Io(value)
	}
}

impl From<Locker> for Error {
	fn from(value: Locker) -> Self {
		Error::Locker(value)
	}
}

#[cfg(feature = "dbus")]
impl From<dbus::Error> for Error {
	fn from(value: dbus::Error) -> Self {
		Error::DBus(value)
	}
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter) -> ::std::result::Result<(), fmt::Error> {
		f.write_str(error::Error::description(self))
	}
}

impl error::Error for Error {
	fn description(&self) -> &str {
		match self {
			&Error::Io(ref err) =>
				err.description(),

			#[cfg(feature = "dbus")]
			&Error::DBus(ref err) =>
				err.description(),

			&Error::Locker(ref err) => match err {
				&Locker::NoDisplay =>
					"No display found.",

				&Locker::NoVisual =>
					"No proper visual found.",

				&Locker::NoIM =>
					"No proper IM found.",

				&Locker::NoIC =>
					"No proper IC found",
			},

			&Error::Parse =>
				"Parse error.",
		}
	}
}
