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

#[derive(Clone, Default, Debug)]
pub struct Saver(pub(super) Arc<RwLock<Data>>);

#[derive(Debug)]
pub(super) struct Data {
	pub timeout:  u32,
	pub throttle: bool,

	pub using: Vec<String>,
	pub table: toml::Table,
}

impl Default for Data {
	fn default() -> Data {
		Data {
			timeout:  5,
			throttle: false,

			using: Default::default(),
			table: Default::default(),
		}
	}
}

impl Saver {
	pub fn load(&self, table: &toml::Table) {
		if let Some(table) = table.get("saver").and_then(|v| v.as_table()) {
			if let Some(value) = super::seconds(table.get("timeout")) {
				self.0.write().unwrap().timeout = value;
			}

			if let Some(value) = table.get("throttle").and_then(|v| v.as_bool()) {
				self.0.write().unwrap().throttle = value;
			}

			if let Some(value) = table.get("use").and_then(|v| v.as_slice()) {
				self.0.write().unwrap().using = value.iter()
					.filter(|v| v.as_str().is_some())
					.map(|v| v.as_str().unwrap().into())
					.collect();
			}

			self.0.write().unwrap().table = table.clone();
		}
	}

	/// The timeout for saver requests.
	pub fn timeout(&self) -> u32 {
		self.0.read().unwrap().timeout
	}

	/// Whether throttling is enabled by default.
	pub fn throttle(&self) -> bool {
		self.0.read().unwrap().throttle
	}

	/// List of savers being used.
	pub fn using(&self) -> Vec<String> {
		self.0.read().unwrap().using.clone()
	}

	/// Get the configuration for a specific saver.
	pub fn get<S: AsRef<str>>(&self, name: S) -> toml::Table {
		self.0.read().unwrap().table.get(name.as_ref())
			.and_then(|v| v.as_table()).cloned().unwrap_or_default()
	}
}
