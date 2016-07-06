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
