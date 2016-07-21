mod locker;
pub use self::locker::Locker;

mod server;
pub use self::server::Server;

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
