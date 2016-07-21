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

use super::OnSuspend;

#[derive(Clone, Default, Debug)]
pub struct Locker(pub(super) Arc<RwLock<Data>>);

#[derive(Debug)]
pub(super) struct Data {
	pub display: Option<String>,
	pub dpms:    bool,

	pub on_suspend: OnSuspend,
}

impl Default for Data {
	fn default() -> Data {
		Data {
			display: None,
			dpms:    true,

			on_suspend: Default::default(),
		}
	}
}

impl Locker {
	pub fn display(&self) -> Option<String> {
		self.0.read().unwrap().display.clone()
	}

	pub fn dpms(&self) -> bool {
		self.0.read().unwrap().dpms
	}

	pub fn on_suspend(&self) -> OnSuspend {
		self.0.read().unwrap().on_suspend
	}
}
