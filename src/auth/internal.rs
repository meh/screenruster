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

use error;
use super::Authenticate;

pub struct Auth {
	password: String,
}

pub fn new(config: toml::Table) -> error::Result<Auth> {
	Ok(Auth {
		password: config.get("password")
			.and_then(|v| v.as_str())
			.map(|s| s.to_string())
			.ok_or(error::auth::Internal::Creation)?,
	})
}

impl Authenticate for Auth {
	fn authenticate(&mut self, _user: &str, password: &str) -> error::Result<bool> {
		Ok(self.password == password)
	}
}
