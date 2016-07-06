use toml;
use pam;

use error;
use super::Authenticate;

pub struct Auth;

pub fn new(_config: toml::Table) -> error::Result<Auth> {
	Ok(Auth)
}

impl Authenticate for Auth {
	fn authenticate(&mut self, user: &str, password: &str) -> error::Result<bool> {
		let mut pam = pam::Authenticator::new("screenruster").ok_or(error::auth::Pam::Creation)?;

		pam.set_credentials(user, password);
		pam.authenticate().and(Ok(true)).or(Ok(false))
	}
}
