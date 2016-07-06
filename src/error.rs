// Copyleft (ↄ) meh. <meh@schizofreni.co> | http://meh.schizofreni.co
//
// This file is part of screenruster.
//
// screenruster is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// screenruster is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with screenruster.  If not, see <http://www.gnu.org/licenses/>.

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
	Grab(Grab),

	#[cfg(feature = "dbus")]
	DBus(dbus::Error),

	Parse,
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum Locker {
	Display,
	Visual,
	IM,
	IC,
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum Grab {
	Conflict,
	Frozen,
	Unmapped,
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

impl From<Grab> for Error {
	fn from(value: Grab) -> Self {
		Error::Grab(value)
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
		match *self {
			Error::Io(ref err) =>
				err.description(),

			#[cfg(feature = "dbus")]
			Error::DBus(ref err) =>
				err.description(),

			Error::Locker(ref err) => match *err {
				Locker::Display =>
					"No display found.",

				Locker::Visual =>
					"No proper visual found.",

				Locker::IM =>
					"No proper IM found.",

				Locker::IC =>
					"No proper IC found",
			},

			Error::Grab(ref err) => match *err {
				Grab::Conflict =>
					"A grab is already present.",

				Grab::Frozen =>
					"The grab is frozen.",

				Grab::Unmapped =>
					"The grabbing window is not mapped.",
			},

			Error::Parse =>
				"Parse error.",
		}
	}
}
