use std::fmt;
use std::error;
use std::io;

use api;
use dbus;

pub type Result<T> = ::std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
	Io(io::Error),
	ContextCreation(api::gl::GliumCreationError<Window>),
	SwapBuffers(api::gl::SwapBuffersError),
	DBus(dbus::Error),
	Parse,
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum Window {
	NoDisplay,
	NoVisual,
	NoContext,
	NoIM,
	NoIC,

	AlreadyPresent,
	MissingExtension,
}

impl From<io::Error> for Error {
	fn from(value: io::Error) -> Self {
		Error::Io(value)
	}
}

impl From<Window> for Error {
	fn from(value: Window) -> Self {
		Error::ContextCreation(api::gl::GliumCreationError::BackendCreationError(value))
	}
}

impl From<api::gl::GliumCreationError<Window>> for Error {
	fn from(value: api::gl::GliumCreationError<Window>) -> Self {
		Error::ContextCreation(value)
	}
}

impl From<api::gl::SwapBuffersError> for Error {
	fn from(value: api::gl::SwapBuffersError) -> Self {
		Error::SwapBuffers(value)
	}
}

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

			&Error::ContextCreation(ref err) =>
				"OpenGL error.",

			&Error::SwapBuffers(ref err) =>
				err.description(),

			&Error::DBus(ref err) =>
				err.description(),

			&Error::Parse =>
				"Parse error.",
		}
	}
}
