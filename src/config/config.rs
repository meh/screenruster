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

use std::fs::File;
use std::path::{Path, PathBuf};
use std::io::Read;
use std::sync::{Arc, RwLock};

use toml;
use xdg;

use error;
use super::{Locker, Interface, Timer, Auth, Saver, OnSuspend};

#[derive(Clone, Debug, Default)]
pub struct Config {
	path: Arc<RwLock<Option<PathBuf>>>,

	locker: Locker,
	interface: Interface,
	timer:  Timer,
	auth:   Auth,
	saver:  Saver,
}

impl Config {
	pub fn load<T: AsRef<Path>>(path: Option<T>) -> error::Result<Config> {
		let config = Config::default();
		config.reload(path)?;

		Ok(config)
	}

	pub fn reset(&self) {
		*self.locker.0.write().unwrap() = Default::default();
		*self.interface.0.write().unwrap() = Default::default();
		*self.timer.0.write().unwrap()  = Default::default();
		*self.auth.0.write().unwrap()   = Default::default();
		*self.saver.0.write().unwrap()  = Default::default();
	}

	pub fn reload<T: AsRef<Path>>(&self, path: Option<T>) -> error::Result<()> {
		let path = if let Some(path) = path {
			*self.path.write().unwrap() = Some(path.as_ref().into());
			path.as_ref().into()
		}
		else if let Some(path) = self.path.read().unwrap().clone() {
			path
		}
		else {
			xdg::BaseDirectories::with_prefix("screenruster").unwrap()
				.place_config_file("config.toml").unwrap()
		};

		let table = if let Ok(mut file) = File::open(path) {
			let mut content = String::new();
			file.read_to_string(&mut content).unwrap();

			toml::Parser::new(&content).parse().ok_or(error::Error::Parse)?
		}
		else {
			toml::Table::new()
		};

		// Load `Locker`.
		if let Some(table) = table.get("locker").and_then(|v| v.as_table()) {
			if let Some(value) = table.get("display").and_then(|v| v.as_str()) {
				self.locker.0.write().unwrap().display = Some(value.into());
			}

			if let Some(false) = table.get("dpms").and_then(|v| v.as_bool()) {
				self.locker.0.write().unwrap().dpms = false;
			}

			if let Some(value) = table.get("on-suspend").and_then(|v| v.as_str()) {
				self.locker.0.write().unwrap().on_suspend = match value {
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

		// Load `Interface`.
		if let Some(table) = table.get("interface").and_then(|v| v.as_table()) {
			if let Some(array) = table.get("ignore").and_then(|v| v.as_slice()) {
				self.interface.0.write().unwrap().ignore = array.iter()
					.filter(|v| v.as_str().is_some())
					.map(|v| v.as_str().unwrap().to_string())
					.collect();
			}
		}

		// Load `Timer`.
		if let Some(table) = table.get("timer").and_then(|v| v.as_table()) {
			if let Some(value) = seconds(table.get("beat")) {
				self.timer.0.write().unwrap().beat = value;
			}

			if let Some(value) = seconds(table.get("timeout")) {
				self.timer.0.write().unwrap().timeout = value;
			}

			if let Some(value) = seconds(table.get("lock")) {
				self.timer.0.write().unwrap().lock = Some(value);
			}

			if let Some(value) = seconds(table.get("blank")) {
				self.timer.0.write().unwrap().blank = Some(value);
			}
		}

		// Load `Auth`.
		if let Some(table) = table.get("auth").and_then(|v| v.as_table()) {
			self.auth.0.write().unwrap().table = table.clone();
		}

		// Load `Saver`.
		if let Some(table) = table.get("saver").and_then(|v| v.as_table()) {
			if let Some(value) = seconds(table.get("timeout")) {
				self.saver.0.write().unwrap().timeout = value;
			}

			if let Some(value) = table.get("throttle").and_then(|v| v.as_bool()) {
				self.saver.0.write().unwrap().throttle = value;
			}

			if let Some(value) = table.get("use").and_then(|v| v.as_slice()) {
				self.saver.0.write().unwrap().using = value.iter()
					.filter(|v| v.as_str().is_some())
					.map(|v| v.as_str().unwrap().into())
					.collect();
			}

			self.saver.0.write().unwrap().table = table.clone();
		}

		Ok(())
	}

	pub fn timer(&self) -> Timer {
		self.timer.clone()
	}

	pub fn interface(&self) -> Interface {
		self.interface.clone()
	}

	pub fn locker(&self) -> Locker {
		self.locker.clone()
	}

	pub fn auth(&self) -> Auth {
		self.auth.clone()
	}

	pub fn saver(&self) -> Saver {
		self.saver.clone()
	}
}

fn seconds(value: Option<&toml::Value>) -> Option<u32> {
	macro_rules! try {
		($body:expr) => (
			if let Ok(value) = $body {
				value: u32
			}
			else {
				return None;
			}
		);
	}

	if value.is_none() {
		return None;
	}

	match *value.unwrap() {
		toml::Value::Integer(value) => {
			Some(value as u32)
		}

		toml::Value::Float(value) => {
			Some(value.round() as u32)
		}

		toml::Value::String(ref value) => {
			match value.split(':').collect::<Vec<&str>>()[..] {
				[hours, minutes, seconds] =>
					Some(try!(hours.parse()) * 60 * 60 + try!(minutes.parse()) * 60 + try!(seconds.parse())),

				[minutes, seconds] =>
					Some(try!(minutes.parse()) * 60 + try!(seconds.parse())),

				[seconds] =>
					Some(try!(seconds.parse())),

				_ =>
					None
			}
		}

		_ =>
			None
	}
}
