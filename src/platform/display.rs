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
use platform::Event;

pub struct Display {
	connection: xcb::Connection,
	receiver:   Receiver<Event>,

	screen: i32,
	name:   String,
}

unsafe impl Send for Display { }
unsafe impl Sync for Display { }

impl Display {
	pub fn open(name: Option<String>) -> error::Result<Arc<Display>> {
		let name                 = name.or_else(|| env::var("DISPLAY").ok()).unwrap_or(":0.0".into());
		let (connection, screen) = xcb::Connection::connect(Some(name.as_ref()))?;
		let (sender, receiver)   = sync_channel(1);

		let display = Arc::new(Display {
			connection: connection,
			receiver:   receiver,

			screen: screen,
			name:   name,
		});

		// Drain events into a channel.
		{
			let display = display.clone();

			thread::spawn(move || {
				while let Some(event) = display.wait_for_event() {
					sender.send(Event(event)).unwrap();
				}
			});
		}

		Ok(display)
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

impl AsRef<Receiver<Event>> for Display {
	fn as_ref(&self) -> &Receiver<Event> {
		&self.receiver
	}
}

impl Deref for Display {
	type Target = xcb::Connection;

	fn deref(&self) -> &Self::Target {
		&self.connection
	}
}
