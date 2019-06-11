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

use std::sync::Arc;
use std::ops::Deref;
use std::thread;
use channel;

use xcb;
use xcbu::ewmh;

use crate::error;

pub struct Display {
	connection: ewmh::Connection,

	screen: i32,
	name:   Option<String>,
}

impl Display {
	pub fn open(name: Option<String>) -> error::Result<Arc<Display>> {
		let (connection, screen) = xcb::Connection::connect(name.as_ref().map(AsRef::as_ref))?;
		let connection           = ewmh::Connection::connect(connection).map_err(|(e, _)| e)?;

		Ok(Arc::new(Display {
			connection, screen, name
		}))
	}

	pub fn screen(&self) -> i32 {
		self.screen
	}

	pub fn name(&self) -> Option<&str> {
		self.name.as_ref().map(AsRef::as_ref)
	}

	pub fn screens(&self) -> u8 {
		self.get_setup().roots_len()
	}
}

pub fn sink(display: &Arc<Display>) -> channel::Receiver<xcb::GenericEvent> {
	let (sender, receiver) = channel::bounded(1);
	let display            = display.clone();

	// Drain events into a channel.
	thread::spawn(move || {
		while let Some(event) = display.wait_for_event() {
			sender.send(event).unwrap();
		}
	});

	receiver
}

impl Deref for Display {
	type Target = xcb::Connection;

	fn deref(&self) -> &Self::Target {
		&self.connection
	}
}
