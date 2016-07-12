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
#![feature(mpsc_select, stmt_expr_attributes, box_syntax)]

#[macro_use]
extern crate log;
extern crate env_logger;

extern crate clap;
extern crate xdg;
extern crate toml;
extern crate rand;
extern crate users;
extern crate dbus;

#[cfg(feature = "auth-pam")]
extern crate pam_sys as pam;

extern crate libc;
extern crate x11;

#[macro_use]
extern crate screenruster_saver as api;

use clap::{ArgMatches, Arg, App, SubCommand};

#[macro_use]
mod util;

mod error;

mod config;
use config::Config;

mod locker;
use locker::Locker;

mod auth;
use auth::Auth;

mod server;
use server::Server;

mod timer;
use timer::Timer;

mod saver;

fn main() {
	env_logger::init().unwrap();

	let matches = App::new("screenruster")
		.version(env!("CARGO_PKG_VERSION"))
		.author("meh. <meh@schizofreni.co>")
		.arg(Arg::with_name("config")
			.short("c")
			.long("config")
			.help("Sets a custom configuration file.")
			.takes_value(true))
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
		.get_matches();

	let config = Config::load(&matches).unwrap();

	if let Some(submatches) = matches.subcommand_matches("lock") {
		lock(submatches.clone(), config).unwrap();
	}
	else if let Some(submatches) = matches.subcommand_matches("activate") {
		activate(submatches.clone(), config).unwrap();
	}
	else if let Some(submatches) = matches.subcommand_matches("deactivate") {
		deactivate(submatches.clone(), config).unwrap();
	}
	else if let Some(submatches) = matches.subcommand_matches("inhibit") {
		inhibit(submatches.clone(), config).unwrap();
	}
	else if let Some(submatches) = matches.subcommand_matches("uninhibit") {
		uninhibit(submatches.clone(), config).unwrap();
	}
	else if let Some(submatches) = matches.subcommand_matches("throttle") {
		throttle(submatches.clone(), config).unwrap();
	}
	else if let Some(submatches) = matches.subcommand_matches("unthrottle") {
		unthrottle(submatches.clone(), config).unwrap();
	}
	else {
		daemon(matches.clone(), config).unwrap();
	}
}

fn lock(_matches: ArgMatches, _config: Config) -> error::Result<()> {
	dbus::Connection::get_private(dbus::BusType::Session)?
		.send(dbus::Message::new_method_call(
			"org.gnome.ScreenSaver",
			"/org/gnome/ScreenSaver",
			"org.gnome.ScreenSaver",
			"Lock")?)?;

	Ok(())
}

fn activate(_matches: ArgMatches, _config: Config) -> error::Result<()> {
	dbus::Connection::get_private(dbus::BusType::Session)?
		.send(dbus::Message::new_method_call(
			"org.gnome.ScreenSaver",
			"/org/gnome/ScreenSaver",
			"org.gnome.ScreenSaver",
			"SetActive")?
				.append1(true))?;

	Ok(())
}

fn deactivate(_matches: ArgMatches, _config: Config) -> error::Result<()> {
	dbus::Connection::get_private(dbus::BusType::Session)?
		.send(dbus::Message::new_method_call(
			"org.gnome.ScreenSaver",
			"/org/gnome/ScreenSaver",
			"org.gnome.ScreenSaver",
			"SimulateUserActivity")?)?;

	Ok(())
}

fn inhibit(_matches: ArgMatches, _config: Config) -> error::Result<()> {
	let reply = dbus::Connection::get_private(dbus::BusType::Session)?
		.send_with_reply_and_block(dbus::Message::new_method_call(
			"org.gnome.ScreenSaver",
			"/org/gnome/ScreenSaver",
			"org.gnome.ScreenSaver",
			"Inhibit")?
				.append2("screenruster", "requested by user")
			, 5_000)?;

	println!("{}", reply.get1::<u32>().unwrap());

	Ok(())
}

fn uninhibit(matches: ArgMatches, _config: Config) -> error::Result<()> {
	dbus::Connection::get_private(dbus::BusType::Session)?
		.send(dbus::Message::new_method_call(
			"org.gnome.ScreenSaver",
			"/org/gnome/ScreenSaver",
			"org.gnome.ScreenSaver",
			"UnInhibit")?
				.append1(matches.value_of("COOKIE").unwrap().parse::<u32>().unwrap()))?;

	Ok(())
}

fn throttle(_matches: ArgMatches, _config: Config) -> error::Result<()> {
	let reply = dbus::Connection::get_private(dbus::BusType::Session)?
		.send_with_reply_and_block(dbus::Message::new_method_call(
			"org.gnome.ScreenSaver",
			"/org/gnome/ScreenSaver",
			"org.gnome.ScreenSaver",
			"Throttle")?
				.append2("screenruster", "requested by user")
			, 5_000)?;

	println!("{}", reply.get1::<u32>().unwrap());

	Ok(())
}

fn unthrottle(matches: ArgMatches, _config: Config) -> error::Result<()> {
	dbus::Connection::get_private(dbus::BusType::Session)?
		.send(dbus::Message::new_method_call(
			"org.gnome.ScreenSaver",
			"/org/gnome/ScreenSaver",
			"org.gnome.ScreenSaver",
			"UnThrottle")?
				.append1(matches.value_of("COOKIE").unwrap().parse::<u32>().unwrap()))?;

	Ok(())
}

fn daemon(_matches: ArgMatches, config: Config) -> error::Result<()> {
	use std::time::Instant;
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

	let timer  = Timer::spawn(config.timer().clone())?;
	let auth   = Auth::spawn(config.auth().clone())?;
	let server = Server::spawn(config.server().clone())?;
	let locker = Locker::spawn(config.clone())?;

	let mut locked  = None: Option<Instant>;
	let mut started = None: Option<Instant>;
	let mut blanked = None: Option<Instant>;

	let mut inhibitors = HashSet::new();
	let mut throttlers = HashSet::new();

	macro_rules! act {
		(blank) => (
			blanked = Some(Instant::now());
			locker.power(false).unwrap();
		);

		(unblank) => (
			blanked = None;
			locker.power(true).unwrap();
		);

		(start) => (
			started = Some(Instant::now());
			locker.start().unwrap();
			server.signal(server::Signal::Active(true)).unwrap();
		);

		(lock) => (
			locked = Some(Instant::now());
			locker.lock().unwrap();
		);

		(stop) => (
			started = None;
			locked  = None;
			locker.stop().unwrap();
			timer.restart().unwrap();
			server.signal(server::Signal::Active(false)).unwrap();
		);

		(auth < $value:expr) => (
			server.signal(server::Signal::AuthenticationRequest(true)).unwrap();
			auth.authenticate($value).unwrap();
		);

		(auth success) => (
			locker.auth(true).unwrap();
			server.signal(server::Signal::AuthenticationRequest(false)).unwrap();
		);

		(auth failure) => (
			locker.auth(false).unwrap();
			server.signal(server::Signal::AuthenticationRequest(false)).unwrap();
		);
	}

	// XXX: select! is icky, this works around shadowing the outer name
	let l = &*locker;
	let a = &*auth;
	let s = &*server;
	let t = &*timer;

	loop {
		select! {
			// Locker events.
			event = l.recv() => {
				match event.unwrap() {
					// Try authorization.
					locker::Response::Password(pwd) => {
						act!(auth < pwd);
					}

					// On system activity.
					locker::Response::Activity => {
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
					server::Request::Lock => {
						if started.is_none() {
							act!(start);
						}

						if locked.is_none() {
							act!(lock);
						}
					}

					// Cycle is unsupported.
					server::Request::Cycle => (),

					server::Request::SimulateUserActivity => {
						locker.activity().unwrap();
					}

					server::Request::Inhibit { .. } => {
						server.response(server::Response::Inhibit(insert(&mut inhibitors))).unwrap();
					}

					server::Request::UnInhibit(cookie) => {
						inhibitors.remove(&cookie);
					}

					server::Request::Throttle { .. } => {
						if throttlers.is_empty() && !config.saver().throttle() {
							locker.throttle(true).unwrap();
						}

						server.response(server::Response::Throttle(insert(&mut throttlers))).unwrap();
					}

					server::Request::UnThrottle(cookie) => {
						throttlers.remove(&cookie);

						if throttlers.is_empty() && !config.saver().throttle() {
							locker.throttle(false).unwrap();
						}
					}

					server::Request::SetActive(active) => {
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

					server::Request::GetActive => {
						server.response(server::Response::Active(started.is_some())).unwrap();
					}

					server::Request::GetActiveTime => {
						timer.report(GET_ACTIVE_TIME).unwrap();
					}

					server::Request::GetSessionIdle => {
						timer.report(GET_SESSION_IDLE).unwrap();
					}

					server::Request::GetSessionIdleTime => {
						timer.report(GET_SESSION_IDLE_TIME).unwrap();
					}
				}
			},

			// Timer events.
			event = t.recv() => {
				match event.unwrap() {
					timer::Response::Report { id: GET_ACTIVE_TIME, started, .. } => {
						server.response(server::Response::ActiveTime(started.map_or(0, |i| i.elapsed().as_secs()))).unwrap();
					}

					timer::Response::Report { id: GET_SESSION_IDLE, idle, .. } => {
						server.response(server::Response::SessionIdle(idle.elapsed().as_secs() >= 5)).unwrap();
					}

					timer::Response::Report { id: GET_SESSION_IDLE_TIME, idle, .. } => {
						server.response(server::Response::SessionIdleTime(idle.elapsed().as_secs())).unwrap();
					}

					timer::Response::Report { .. } => {
						unreachable!();
					}

					timer::Response::Heartbeat => {
						locker.sanitize().unwrap();
					}

					timer::Response::Start => {
						if inhibitors.is_empty() {
							act!(start);
						}
						else {
							timer.reset(timer::Event::Idle).unwrap();
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
							timer.reset(timer::Event::Blank).unwrap();
						}
					}
				}
			}
		}
	}
}
