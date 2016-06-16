use std::fs::File;
use std::io::{self, Read};
use std::path::PathBuf;
use std::collections::HashMap;

use toml;
use clap::ArgMatches;
use xdg;

use error;

#[derive(Clone, Debug)]
pub struct Config {
	server: Server,
	window: Window,
	auth:   Auth,
	saver:  Saver,
}

#[derive(Clone, Debug)]
pub struct Server {

}

#[derive(Clone, Debug)]
pub struct Window {
	pub timeout: i16,
	pub interval: i16,
}

#[derive(Clone, Debug)]
pub struct Auth {
	using:  Vec<String>,
	config: HashMap<String, toml::Table>,
}

#[derive(Clone, Debug)]
pub struct Saver {
	using:  Vec<String>,
	config: HashMap<String, toml::Table>,
}

impl Config {
	pub fn load(matches: &ArgMatches) -> error::Result<Config> {
		let path = if let Some(path) = matches.value_of("config") {
			path.into()
		}
		else {
			xdg::BaseDirectories::with_prefix("screenruster").unwrap()
				.place_config_file("config.toml").unwrap()
		};

		let mut file    = File::open(path).unwrap();
		let mut content = String::new();
		file.read_to_string(&mut content).unwrap();

		let table = toml::Parser::new(&content).parse().ok_or(error::Error::Parse)?;

		Ok(Config {
			server: {
				Server { }
			},

			window: {
				Window {
					timeout:  table.get("timeout").and_then(|v| v.as_integer()).map(|v| v as i16).unwrap_or(360),
					interval: table.get("interval").and_then(|v| v.as_integer()).map(|v| v as i16).unwrap_or(360),
				}
			},

			auth: {
				Auth {
					using:  Vec::new(),
					config: HashMap::new(),
				}
			},

			saver: {
				Saver {
					using:  Vec::new(),
					config: HashMap::new(),
				}
			},
		})
	}

	pub fn server(&self) -> Server {
		self.server.clone()
	}

	pub fn window(&self) -> Window {
		self.window.clone()
	}

	pub fn auth<S: AsRef<str>>(&self, name: S) -> toml::Table {
		if let Some(conf) = self.auth.config.get(name.as_ref()) {
			conf.clone()
		}
		else {
			toml::Table::new()
		}
	}

	pub fn saver<S: AsRef<str>>(&self, name: S) -> toml::Table {
		if let Some(conf) = self.saver.config.get(name.as_ref()) {
			conf.clone()
		}
		else {
			toml::Table::new()
		}
	}
}
