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

use toml;

mod locker;
pub use self::locker::Locker;

mod interface;
pub use self::interface::Interface;

mod timer;
pub use self::timer::Timer;

mod auth;
pub use self::auth::Auth;

mod saver;
pub use self::saver::Saver;

mod config;
pub use self::config::Config;

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
