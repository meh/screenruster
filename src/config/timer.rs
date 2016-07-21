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

use std::sync::{Arc, RwLock};

#[derive(Clone, Default, Debug)]
pub struct Timer(pub(super) Arc<RwLock<Data>>);

#[derive(Debug)]
pub(super) struct Data {
	pub beat:    u32,
	pub timeout: u32,
	pub lock:    Option<u32>,
	pub blank:   Option<u32>,
}

impl Default for Data {
	fn default() -> Data {
		Data {
			beat:    30,
			timeout: 360,
			lock:    None,
			blank:   None,
		}
	}
}

impl Timer {
	pub fn beat(&self) -> u32 {
		self.0.read().unwrap().beat
	}

	pub fn timeout(&self) -> u32 {
		self.0.read().unwrap().timeout
	}

	pub fn lock(&self) -> Option<u32> {
		self.0.read().unwrap().lock
	}

	pub fn blank(&self) -> Option<u32> {
		self.0.read().unwrap().blank
	}
}
