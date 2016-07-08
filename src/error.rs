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
	Auth(Auth),
	Parse,

	#[cfg(feature = "dbus")]
	DBus(dbus::Error),
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

#[derive(Clone, Debug)]
pub enum Auth {
	UnknownUser,

	#[cfg(feature = "auth-internal")]
	Internal(auth::Internal),

	#[cfg(feature = "auth-pam")]
	Pam(auth::Pam),
}

pub mod auth {
	#[cfg(feature = "auth-pam")]
	use pam;

	#[derive(Clone, Debug)]
	#[cfg(feature = "auth-internal")]
	pub enum Internal {
		MissingPassword,
	}

	#[derive(Clone, Debug)]
	#[cfg(feature = "auth-pam")]
	pub struct Pam(pub pam::PamReturnCode);
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

impl From<Auth> for Error {
	fn from(value: Auth) -> Self {
		Error::Auth(value)
	}
}

#[cfg(feature = "auth-internal")]
impl From<auth::Internal> for Error {
	fn from(value: auth::Internal) -> Self {
		Error::Auth(Auth::Internal(value))
	}
}

#[cfg(feature = "auth-pam")]
impl From<auth::Pam> for Error {
	fn from(value: auth::Pam) -> Self {
		Error::Auth(Auth::Pam(value))
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

			Error::Auth(ref err) => match *err {
				Auth::UnknownUser =>
					"Unknown user.",

				#[cfg(feature = "auth-internal")]
				Auth::Internal(ref err) => match *err {
					auth::Internal::MissingPassword =>
						"Missing internal password.",
				},

				#[cfg(feature = "auth-pam")]
				Auth::Pam(auth::Pam(_code)) =>
					"PAM error.",
			},

			Error::Parse =>
				"Parse error.",
		}
	}
}
