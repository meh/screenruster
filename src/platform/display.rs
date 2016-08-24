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
use std::env;
use std::ops::Deref;
use std::thread;
use std::sync::mpsc::{Receiver, sync_channel};

use xcb;

use error;

pub struct Display {
	connection: xcb::Connection,

	screen: i32,
	name:   String,
}

impl Display {
	pub fn open(name: Option<String>) -> error::Result<Arc<Display>> {
		let name                 = name.or_else(|| env::var("DISPLAY").ok()).unwrap_or(":0.0".into());
		let (connection, screen) = xcb::Connection::connect(Some(name.as_ref()))?;

		Ok(Arc::new(Display {
			connection: connection,

			screen: screen,
			name:   name,
		}))
	}

	pub fn screen(&self) -> i32 {
		self.screen
	}

	pub fn name(&self) -> &str {
		&self.name
	}

	pub fn screens(&self) -> u8 {
		self.get_setup().roots_len()
	}
}

pub fn sink(display: &Arc<Display>) -> Receiver<xcb::GenericEvent> {
	let (sender, receiver) = sync_channel(1);
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
