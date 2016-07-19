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
use std::io::Read;
use std::collections::HashSet;

use toml;
use clap::ArgMatches;
use xdg;

use error;

#[derive(Clone, Debug)]
pub struct Config {
	locker: Locker,
	server: Server,
	timer:  Timer,
	auth:   Auth,
	saver:  Saver,
}

#[derive(Clone, Debug)]
pub struct Timer {
	pub beat:    u32,
	pub timeout: u32,
	pub lock:    Option<u32>,
	pub blank:   Option<u32>,
}

impl Default for Timer {
	fn default() -> Timer {
		Timer {
			beat:    30,
			timeout: 360,
			lock:    None,
			blank:   None,
		}
	}
}

#[derive(Clone, Debug)]
pub struct Server {
	pub ignore: HashSet<String>,
}

impl Default for Server {
	fn default() -> Server {
		Server {
			ignore: HashSet::new(),
		}
	}
}

#[derive(Clone, Debug)]
pub struct Locker {
	pub display: Option<String>,
	pub dpms:    bool,

	pub on_suspend: OnSuspend,
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum OnSuspend {
	Ignore,
	UseSystemTime,
	Activate,
	Lock,
}

impl Default for OnSuspend {
	fn default() -> OnSuspend {
		OnSuspend::Ignore
	}
}

impl Default for Locker {
	fn default() -> Locker {
		Locker {
			display: None,
			dpms:    true,

			on_suspend: Default::default(),
		}
	}
}

#[derive(Clone, Default, Debug)]
pub struct Auth(toml::Table);

#[derive(Clone, Default, Debug)]
pub struct Saver(toml::Table);

impl Config {
	pub fn load(matches: &ArgMatches) -> error::Result<Config> {
		let path = if let Some(path) = matches.value_of("config") {
			path.into()
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

		Ok(Config {
			locker: {
				let mut config = Locker::default();

				if let Some(table) = table.get("locker").and_then(|v| v.as_table()) {
					if let Some(value) = table.get("display").and_then(|v| v.as_str()) {
						config.display = Some(value.into());
					}

					if let Some(false) = table.get("dpms").and_then(|v| v.as_bool()) {
						config.dpms = false;
					}

					if let Some(value) = table.get("on-suspend").and_then(|v| v.as_str()) {
						config.on_suspend = match value {
							"use-system-time" =>
								OnSuspend::UseSystemTime,

							"lock" =>
								OnSuspend::Lock,

							"activate" =>
								OnSuspend::Activate,

							_ =>
								Default::default()
						}
					}
				}

				config
			},

			server: {
				let mut config = Server::default();

				if let Some(table) = table.get("server").and_then(|v| v.as_table()) {
					if let Some(array) = table.get("ignore").and_then(|v| v.as_slice()) {
						config.ignore = array.iter().filter(|v| v.as_str().is_some()).map(|v| v.as_str().unwrap().to_string()).collect();
					}
				}

				config
			},

			timer: {
				let mut config = Timer::default();

				if let Some(table) = table.get("timer").and_then(|v| v.as_table()) {
					if let Some(value) = table.get("beat").and_then(|v| v.as_integer()) {
						config.beat = value as u32;
					}

					if let Some(value) = table.get("timeout").and_then(|v| v.as_integer()) {
						config.timeout = value as u32;
					}

					if let Some(value) = table.get("lock").and_then(|v| v.as_integer()) {
						config.lock = Some(value as u32);
					}

					if let Some(value) = table.get("blank").and_then(|v| v.as_integer()) {
						config.blank = Some(value as u32);
					}
				}

				config
			},

			auth: {
				if let Some(table) = table.get("auth").and_then(|v| v.as_table()) {
					Auth(table.clone())
				}
				else {
					Auth(toml::Table::new())
				}
			},

			saver: {
				if let Some(table) = table.get("saver").and_then(|v| v.as_table()) {
					Saver(table.clone())
				}
				else {
					Saver(toml::Table::new())
				}
			},
		})
	}

	pub fn timer(&self) -> &Timer {
		&self.timer
	}

	pub fn server(&self) -> &Server {
		&self.server
	}

	pub fn locker(&self) -> &Locker {
		&self.locker
	}

	pub fn auth(&self) -> &Auth {
		&self.auth
	}

	pub fn saver(&self) -> &Saver {
		&self.saver
	}
}

impl Auth {
	/// Get the configuration for a specific authorization module.
	pub fn get<S: AsRef<str>>(&self, name: S) -> toml::Table {
		self.0.get(name.as_ref()).and_then(|v| v.as_table()).cloned().unwrap_or_default()
	}
}

impl Saver {
	/// List of savers being used.
	pub fn using(&self) -> Vec<&str> {
		self.0.get("use").and_then(|v| v.as_slice())
			.unwrap_or(&[])
			.iter()
			.filter(|v| v.as_str().is_some())
			.map(|v| v.as_str().unwrap())
			.collect()
	}

	/// Whether throttling is enabled by default.
	pub fn throttle(&self) -> bool {
		self.0.get("throttle").and_then(|v| v.as_bool()).unwrap_or(false)
	}

	/// Get the configuration for a specific saver.
	pub fn get<S: AsRef<str>>(&self, name: S) -> toml::Table {
		self.0.get(name.as_ref()).and_then(|v| v.as_table()).cloned().unwrap_or_default()
	}
}
