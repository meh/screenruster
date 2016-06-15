pub trait Authenticate {
	fn authenticate<U: AsRef<str>, P: AsRef<str>>(user: U, password: P) -> Result<bool>;
}

#[cfg(feature = "auth-pam")]
mod pam;
#[cfg(feature = "auth-pam")]
pub use self::pam::Pam;
