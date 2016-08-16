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

use toml;

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
	pub fn load(&self, table: &toml::Table) {
		if let Some(table) = table.get("locker").and_then(|v| v.as_table()) {
			if let Some(value) = table.get("display").and_then(|v| v.as_str()) {
				self.0.write().unwrap().display = Some(value.into());
			}

			if let Some(false) = table.get("dpms").and_then(|v| v.as_bool()) {
				self.0.write().unwrap().dpms = false;
			}

			if let Some(value) = table.get("on-suspend").and_then(|v| v.as_str()) {
				self.0.write().unwrap().on_suspend = match value {
					"use-system-time" =>
						OnSuspend::UseSystemTime,

					"lock" =>
						OnSuspend::Lock,

					"activate" =>
						OnSuspend::Activate,

					_ =>
						Default::default()
				};
			}
		}
	}

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
