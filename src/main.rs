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

#![feature(type_ascription, question_mark, associated_type_defaults)]
#![feature(mpsc_select, stmt_expr_attributes, box_syntax, slice_patterns)]
#![feature(pub_restricted)]

#[macro_use]
extern crate log;
extern crate env_logger;

extern crate clap;
use clap::{ArgMatches, Arg, App, SubCommand};

extern crate xdg;
extern crate toml;
extern crate rand;
extern crate users;
extern crate dbus;

#[cfg(feature = "auth-pam")]
extern crate pam_sys as pam;

extern crate libc;
extern crate xcb;
extern crate xcb_util;
extern crate xkbcommon;

#[macro_use]
extern crate screenruster_saver as api;

mod error;

mod config;
use config::Config;

mod platform;
mod saver;

mod preview;
use preview::Preview;

mod locker;
use locker::Locker;

mod auth;
use auth::Auth;

mod interface;
use interface::Interface;

mod timer;
use timer::Timer;

fn main() {
	env_logger::init().unwrap();

	let mut app = App::new("screenruster")
		.version(env!("CARGO_PKG_VERSION"))
		.author("meh. <meh@schizofreni.co>")
		.subcommand(SubCommand::with_name("reload")
			.about("Reload the configuration file.")
			.arg(Arg::with_name("config")
				.short("c")
				.long("config")
				.help("The path to the configuration file to use as replacement.")
				.takes_value(true)))
		.subcommand(SubCommand::with_name("lock")
			.about("Lock the screen."))
		.subcommand(SubCommand::with_name("activate")
			.about("Activate the screen saver."))
		.subcommand(SubCommand::with_name("deactivate")
			.about("Deactivate the screen saver like there was user input."))
		.subcommand(SubCommand::with_name("inhibit")
			.about("Inhibit the screen saver until uninhibit is called."))
		.subcommand(SubCommand::with_name("uninhibit")
			.about("Uninhibit a previous inhibition.")
			.arg(Arg::with_name("COOKIE")
				.required(true)
				.index(1)
				.help("The previously returned cookie.")))
		.subcommand(SubCommand::with_name("throttle")
			.about("Throttle the screen saver until unthrottle is called."))
		.subcommand(SubCommand::with_name("unthrottle")
			.about("Unthrottle a previous throttle.")
			.arg(Arg::with_name("COOKIE")
				.required(true)
				.index(1)
				.help("The previously returned cookie.")))
		.subcommand(SubCommand::with_name("suspend")
			.about("Prepare the saver for suspension."))
		.subcommand(SubCommand::with_name("resume")
			.about("Prepare the saver for resuming from suspension.")
			.arg(Arg::with_name("COOKIE")
				.required(true)
				.index(1)
				.help("The previously returned cookie.")))
		.subcommand(SubCommand::with_name("preview")
			.about("Preview a saver.")
			.arg(Arg::with_name("config")
				.short("c")
				.long("config")
				.help("The path to the configuration file.")
				.takes_value(true))
			.arg(Arg::with_name("SAVER")
				.required(true)
				.index(1)
				.help("The saver name.")))
		.subcommand(SubCommand::with_name("daemon")
			.about("Start the daemon.")
			.arg(Arg::with_name("config")
				.short("c")
				.long("config")
				.help("The path to the configuration file.")
				.takes_value(true)));

	let matches = app.clone().get_matches();

	exit(match matches.subcommand() {
		("reload", Some(submatches)) =>
			Interface::reload(submatches.value_of("config")),

		("lock", Some(_)) =>
			Interface::lock(),

		("activate", Some(_)) =>
			Interface::activate(),

		("deactivate", Some(_)) =>
			Interface::deactivate(),

		("inhibit", Some(_)) =>
			Interface::inhibit().map(|v| println!("{}", v)),

		("uninhibit", Some(submatches)) =>
			Interface::uninhibit(submatches.value_of("COOKIE").unwrap().parse::<u32>().unwrap()),

		("throttle", Some(_)) =>
			Interface::throttle().map(|v| println!("{}", v)),

		("unthrottle", Some(submatches)) =>
			Interface::unthrottle(submatches.value_of("COOKIE").unwrap().parse::<u32>().unwrap()),

		("suspend", Some(_)) =>
			Interface::suspend().map(|v| println!("{}", v)),

		("resume", Some(submatches)) =>
			Interface::resume(submatches.value_of("COOKIE").unwrap().parse::<u32>().unwrap()),

		("preview", Some(submatches)) =>
			preview(submatches),

		("daemon", Some(submatches)) =>
			daemon(submatches),

		_ =>
			app.print_help().map_err(|e| e.into())
	});
}

fn exit<T>(value: error::Result<T>) -> T {
	use std::io::Write;
	use error::Error;

	macro_rules! error {
		($code:expr, $message:expr) => (
			error!($code, "{}", $message);
		);

		($code:expr, $message:expr, $($rest:tt)*) => ({
			write!(&mut ::std::io::stderr(), "ERROR: ").unwrap();
			writeln!(&mut ::std::io::stderr(), $message, $($rest)*).unwrap();
			std::process::exit($code);
		});
	}

	match value {
		Ok(value) =>
			value,

		Err(error) => match error {
			Error::Parse =>
				error!(1, "The configuration file has a syntax error."),

			Error::DBus(error::DBus::AlreadyRegistered) =>
				error!(10, "Another screen saver is currently running."),

			Error::DBus(_) =>
				error!(11, "The daemon is not running."),

			ref err =>
				error!(255, err),
		}
	}
}

fn preview(matches: &ArgMatches) -> error::Result<()> {
	let config  = Config::load(matches.value_of("config"))?;
	let preview = Preview::spawn(matches.value_of("SAVER").unwrap(), config)?;

	loop {
		match preview.recv().unwrap() {
			preview::Response::Done(..) => {
				break;
			}
		}
	}

	Ok(())
}

fn daemon(matches: &ArgMatches) -> error::Result<()> {
	use std::time::{Instant, SystemTime};
	use std::collections::HashSet;
	use rand::{self, Rng};

	// Timer report IDs.
	const GET_ACTIVE_TIME:       u64 = 1;
	const GET_SESSION_IDLE:      u64 = 2;
	const GET_SESSION_IDLE_TIME: u64 = 3;

	// How many seconds to wait before acting on an Activity after one was
	// already acted upon.
	const ACTIVATION: u64 = 1;

	fn insert(set: &mut HashSet<u32>) -> u32 {
		loop {
			let cookie = rand::thread_rng().gen();

			if set.contains(&cookie) {
				continue;
			}

			set.insert(cookie);

			return cookie;
		}
	}

	let config    = Config::load(matches.value_of("config"))?;
	let timer     = Timer::spawn(config.timer())?;
	let auth      = Auth::spawn(config.auth())?;
	let interface = Interface::spawn(config.interface())?;
	let locker    = Locker::spawn(config.clone())?;

	let mut locked    = None: Option<Instant>;
	let mut started   = None: Option<Instant>;
	let mut blanked   = None: Option<Instant>;
	let mut suspended = None: Option<SystemTime>;

	let mut inhibitors = HashSet::new();
	let mut throttlers = HashSet::new();
	let mut suspenders = HashSet::new();

	macro_rules! act {
		(suspend) => (
			act!(suspend SystemTime::now())
		);

		(suspend $time:expr) => (
			if suspenders.is_empty() && suspended.is_none() {
				timer.suspend($time).unwrap();
			}
		);

		(resume) => (
			if suspenders.is_empty() && suspended.is_some() {
				if blanked.is_some() {
					act!(unblank);
				}

				timer.resume().unwrap();
			}
		);

		(blank) => (
			blanked = Some(Instant::now());

			locker.power(false).unwrap();
			timer.blanked().unwrap();
		);

		(unblank) => (
			blanked = None;

			locker.power(true).unwrap();
			timer.unblanked().unwrap();
		);

		(start) => (
			started = Some(Instant::now());

			locker.start().unwrap();
			interface.signal(interface::Signal::Active(true)).unwrap();
			timer.started().unwrap();
		);

		(lock) => (
			locked = Some(Instant::now());

			locker.lock().unwrap();
			timer.locked().unwrap();
		);

		(stop) => (
			locker.stop().unwrap();
		);

		(stopped) => (
			started = None;
			locked  = None;

			interface.signal(interface::Signal::Active(false)).unwrap();
			timer.stopped().unwrap();
		);

		(auth < $value:expr) => (
			interface.signal(interface::Signal::AuthenticationRequest(true)).unwrap();
			auth.authenticate($value).unwrap();
		);

		(auth success) => (
			locker.auth(true).unwrap();
			interface.signal(interface::Signal::AuthenticationRequest(false)).unwrap();
		);

		(auth failure) => (
			locker.auth(false).unwrap();
			interface.signal(interface::Signal::AuthenticationRequest(false)).unwrap();
		);
	}

	// XXX: select! is icky, this works around shadowing the outer name
	let l = &*locker;
	let a = &*auth;
	let s = &*interface;
	let t = &*timer;

	loop {
		select! {
			// Locker events.
			event = l.recv() => {
				match event.unwrap() {
					// Register timeout.
					locker::Response::Timeout(what) => {
						timer.timeout(what).unwrap();
					}

					// On system activity.
					locker::Response::Activity => {
						if suspended.is_some() {
							continue;
						}

						// Always reset the blank timer.
						timer.reset(timer::Event::Blank).unwrap();

						if blanked.is_some() {
							act!(unblank);
						}

						// If the saver has started but the screen is not locked, unlock
						// it, otherwise just reset the timers.
						if let Some(at) = started {
							if locked.is_none() && at.elapsed().as_secs() >= ACTIVATION {
								act!(stop);
							}
						}
						else {
							timer.reset(timer::Event::Idle).unwrap();
						}
					}

					// Try authorization.
					locker::Response::Password(pwd) => {
						act!(auth < pwd);
					}

					locker::Response::Stopped => {
						act!(stopped);
					}
				}
			},

			// Authentication events.
			event = a.recv() => {
				match event.unwrap() {
					auth::Response::Success => {
						info!("authorization: success");

						act!(auth success);
						act!(stop);
					}

					auth::Response::Failure => {
						info!("authorization: failure");

						act!(auth failure);
					}
				}
			},

			// DBus events.
			event = s.recv() => {
				match event.unwrap() {
					interface::Request::Reload(source) => {
						config.reset();
						interface.response(interface::Response::Reload(
							config.reload(source).is_ok())).unwrap();
					}

					interface::Request::Lock => {
						if started.is_none() {
							act!(start);
						}

						if locked.is_none() {
							act!(lock);
						}
					}

					// TODO: Implement cycling.
					interface::Request::Cycle => (),

					interface::Request::SimulateUserActivity => {
						locker.activity().unwrap();
					}

					interface::Request::Inhibit { .. } => {
						interface.response(interface::Response::Inhibit(insert(&mut inhibitors))).unwrap();
					}

					interface::Request::UnInhibit(cookie) => {
						if inhibitors.contains(&cookie) {
							inhibitors.remove(&cookie);
						}
					}

					interface::Request::Throttle { .. } => {
						if throttlers.is_empty() && !config.saver().throttle() {
							locker.throttle(true).unwrap();
						}

						interface.response(interface::Response::Throttle(insert(&mut throttlers))).unwrap();
					}

					interface::Request::UnThrottle(cookie) => {
						if throttlers.contains(&cookie) {
							throttlers.remove(&cookie);

							if throttlers.is_empty() && !config.saver().throttle() {
								locker.throttle(false).unwrap();
							}
						}
					}

					interface::Request::SetActive(active) => {
						if active {
							if started.is_none() {
								act!(start);
							}
						}
						else {
							if started.is_some() && locked.is_none() {
								act!(stop);
							}
						}
					}

					interface::Request::GetActive => {
						interface.response(interface::Response::Active(started.is_some())).unwrap();
					}

					interface::Request::GetActiveTime => {
						timer.report(GET_ACTIVE_TIME).unwrap();
					}

					interface::Request::GetSessionIdle => {
						timer.report(GET_SESSION_IDLE).unwrap();
					}

					interface::Request::GetSessionIdleTime => {
						timer.report(GET_SESSION_IDLE_TIME).unwrap();
					}

					interface::Request::Suspend { .. } => {
						act!(suspend);

						interface.response(interface::Response::Suspend(insert(&mut suspenders))).unwrap();
					}

					interface::Request::Resume(cookie) => {
						if suspenders.contains(&cookie) {
							suspenders.remove(&cookie);

							act!(resume);
						}
					}

					interface::Request::PrepareForSleep(time) => {
						if let Some(time) = time {
							match config.locker().on_suspend() {
								config::OnSuspend::Ignore |
								config::OnSuspend::Activate |
								config::OnSuspend::Lock => (),

								config::OnSuspend::UseSystemTime => {
									act!(suspend time);
								}
							}
						}
						else {
							match config.locker().on_suspend() {
								config::OnSuspend::Ignore => (),

								config::OnSuspend::UseSystemTime => {
									act!(resume);
								}

								config::OnSuspend::Activate => {
									act!(start);
								}

								config::OnSuspend::Lock => {
									act!(start);
									act!(lock);
								}
							}
						}
					}
				}
			},

			// Timer events.
			event = t.recv() => {
				match event.unwrap() {
					timer::Response::Report { id: GET_ACTIVE_TIME, started, .. } => {
						interface.response(interface::Response::ActiveTime(started.map_or(0, |i| i.elapsed().as_secs()))).unwrap();
					}

					timer::Response::Report { id: GET_SESSION_IDLE, idle, .. } => {
						interface.response(interface::Response::SessionIdle(idle.elapsed().as_secs() >= 5)).unwrap();
					}

					timer::Response::Report { id: GET_SESSION_IDLE_TIME, idle, .. } => {
						interface.response(interface::Response::SessionIdleTime(idle.elapsed().as_secs())).unwrap();
					}

					timer::Response::Report { .. } => {
						unreachable!();
					}

					timer::Response::Timeout { id } => {
						locker.timeout(id).unwrap();
					}

					timer::Response::Suspended(time) => {
						suspended = Some(time);
					}

					timer::Response::Resumed => {
						suspended = None;
					}

					timer::Response::Heartbeat(idle) => {
						locker.sanitize().unwrap();

						if idle.elapsed().as_secs() > 5 {
							interface.signal(interface::Signal::SessionIdle(true)).unwrap()
						}
						else {
							interface.signal(interface::Signal::SessionIdle(false)).unwrap()
						}
					}

					timer::Response::Start => {
						if inhibitors.is_empty() {
							act!(start);
						}
						else {
							timer.stopped().unwrap();
						}
					}

					timer::Response::Lock => {
						act!(lock);
					}

					timer::Response::Blank => {
						if inhibitors.is_empty() {
							act!(blank);
						}
						else {
							timer.unblanked().unwrap();
						}
					}
				}
			}
		}
	}
}
