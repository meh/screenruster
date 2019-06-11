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
use std::ffi;

use xcb;
use dbus;
use clap;
use app_dirs;

pub type Result<T> = ::std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
	Io(io::Error),
	Message(String),
	Nul(ffi::NulError),
	Unknown,
	Parse,

	X(X),
	DBus(DBus),
	Cli(clap::Error),
	Directory(app_dirs::AppDirsError),
	Grab(Grab),
	Auth(Auth),
}

#[derive(Debug)]
pub enum X {
	MissingExtension,
	Request(u8, u8),
	Connection(xcb::ConnError),
}

#[derive(Debug)]
pub enum DBus {
	AlreadyRegistered,
	Internal(dbus::Error),
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

impl From<ffi::NulError> for Error {
	fn from(value: ffi::NulError) -> Self {
		Error::Nul(value)
	}
}

impl From<String> for Error {
	fn from(value: String) -> Self {
		Error::Message(value)
	}
}

impl From<()> for Error {
	fn from(_value: ()) -> Self {
		Error::Unknown
	}
}

impl From<X> for Error {
	fn from(value: X) -> Error {
		Error::X(value)
	}
}

impl From<xcb::ConnError> for Error {
	fn from(value: xcb::ConnError) -> Error {
		Error::X(X::Connection(value))
	}
}

impl<T> From<xcb::Error<T>> for Error {
	fn from(value: xcb::Error<T>) -> Error {
		Error::X(X::Request(value.response_type(), value.error_code()))
	}
}

impl From<dbus::Error> for Error {
	fn from(value: dbus::Error) -> Self {
		Error::DBus(DBus::Internal(value))
	}
}

impl From<clap::Error> for Error {
	fn from(value: clap::Error) -> Self {
		Error::Cli(value)
	}
}

impl From<DBus> for Error {
	fn from(value: DBus) -> Self {
		Error::DBus(value)
	}
}

impl From<app_dirs::AppDirsError> for Error {
	fn from(value: app_dirs::AppDirsError) -> Self {
		Error::Directory(value)
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

			Error::Nul(ref err) =>
				err.description(),

			Error::Message(ref msg) =>
				msg.as_ref(),

			Error::Unknown =>
				"Unknown error.",

			Error::Parse =>
				"Parse error.",

			Error::X(ref err) => match *err {
				X::Request(..) =>
					"An X request failed.",

				X::MissingExtension =>
					"A required X extension is missing.",

				X::Connection(..) =>
					"Connection to the X display failed.",
			},

			Error::DBus(ref err) => match *err {
				DBus::AlreadyRegistered =>
					"The name has already been registered.",

				DBus::Internal(ref err) =>
					err.description(),
			},

			Error::Cli(ref err) =>
				err.description(),

			Error::Directory(ref err) =>
				err.description(),

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
		}
	}
}
