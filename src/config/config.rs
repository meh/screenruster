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
use super::{Locker, Interface, Timer, Auth, Saver};

#[derive(Clone, Debug, Default)]
pub struct Config {
	path: Arc<RwLock<Option<PathBuf>>>,

	locker:    Locker,
	interface: Interface,
	timer:     Timer,
	auth:      Auth,
	saver:     Saver,
}

impl Config {
	pub fn load<T: AsRef<Path>>(path: Option<T>) -> error::Result<Config> {
		let config = Config::default();
		config.reload(path)?;

		Ok(config)
	}

	pub fn reset(&self) {
		*self.locker.0.write().unwrap()    = Default::default();
		*self.interface.0.write().unwrap() = Default::default();
		*self.timer.0.write().unwrap()     = Default::default();
		*self.auth.0.write().unwrap()      = Default::default();
		*self.saver.0.write().unwrap()     = Default::default();
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
			file.read_to_string(&mut content)?;

			toml::Parser::new(&content).parse().ok_or(error::Error::Parse)?
		}
		else {
			toml::Table::new()
		};

		self.locker.load(&table);
		self.interface.load(&table);
		self.timer.load(&table);
		self.auth.load(&table);
		self.saver.load(&table);

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
