use std::fs::File;
use std::io::Read;
use std::collections::HashMap;

use toml;
use clap::ArgMatches;
use xdg;

use error;

#[derive(Clone, Debug)]
pub struct Config {
	timer:  Timer,
	server: Server,
	locker: Locker,
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

}

#[derive(Clone, Debug)]
pub struct Locker {
	pub display: Option<String>,
	pub dpms:    bool,
}

impl Default for Locker {
	fn default() -> Locker {
		Locker {
			display: None,
			dpms:    true,
		}
	}
}

#[derive(Clone, Default, Debug)]
pub struct Auth {
	using: Vec<String>,
	table: HashMap<String, toml::Table>,
}

#[derive(Clone, Default, Debug)]
pub struct Saver {
	using: Vec<String>,
	table: HashMap<String, toml::Table>,
}

impl Config {
	pub fn load(matches: &ArgMatches) -> error::Result<Config> {
		let path = if let Some(path) = matches.value_of("config") {
			path.into()
		}
		else {
			xdg::BaseDirectories::with_prefix("screenruster").unwrap()
				.place_config_file("config.config").unwrap()
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

			server: {
				Server { }
			},

			locker: {
				let mut config = Locker::default();

				if let Some(table) = table.get("locker").and_then(|v| v.as_table()) {
					if let Some(value) = table.get("display").and_then(|v| v.as_str()) {
						config.display = Some(value.into());
					}

					if let Some(false) = table.get("dpms").and_then(|v| v.as_bool()) {
						config.dpms = false;
					}
				}

				config
			},

			auth: {
				let mut config = Auth::default();

				if let Some(table) = table.get("auth").and_then(|v| v.as_table()) {
					if let Some(list) = table.get("use").and_then(|v| v.as_slice()) {
						for using in list {
							if let Some(name) = using.as_str() {
								config.using.push(name.into());
								config.table.insert(name.into(), table.get(name).and_then(|v| v.as_table()).cloned().unwrap_or_default());
							}
						}
					}
				}

				config
			},

			saver: {
				let mut config = Saver::default();

				if let Some(table) = table.get("saver").and_then(|v| v.as_table()) {
					if let Some(list) = table.get("use").and_then(|v| v.as_slice()) {
						for using in list {
							if let Some(name) = using.as_str() {
								config.using.push(name.into());
								config.table.insert(name.into(), table.get(name).and_then(|v| v.as_table()).cloned().unwrap_or_default());
							}
						}
					}
				}

				config
			},
		})
	}

	pub fn timer(&self) -> Timer {
		self.timer.clone()
	}

	pub fn server(&self) -> Server {
		self.server.clone()
	}

	pub fn locker(&self) -> Locker {
		self.locker.clone()
	}

	pub fn auth<S: AsRef<str>>(&self, name: S) -> toml::Table {
		if let Some(conf) = self.auth.table.get(name.as_ref()) {
			conf.clone()
		}
		else {
			toml::Table::new()
		}
	}

	pub fn auths(&self) -> &[String] {
		&self.auth.using[..]
	}

	pub fn saver<S: AsRef<str>>(&self, name: S) -> toml::Table {
		if let Some(conf) = self.saver.table.get(name.as_ref()) {
			conf.clone()
		}
		else {
			toml::Table::new()
		}
	}

	pub fn savers(&self) -> &[String] {
		&self.saver.using[..]
	}
}
