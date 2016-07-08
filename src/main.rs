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

#[cfg(feature = "dbus")]
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
		.version("0.1")
		.author("meh. <meh@schizofreni.co>")
		.arg(Arg::with_name("config")
			.short("c")
			.long("config")
			.help("Sets a custom configuration file.")
			.takes_value(true))
		.subcommand(SubCommand::with_name("lock")
			.about("Lock the screen.")
			.version("0.1"))
		.get_matches();

	let config = Config::load(&matches).unwrap();

	if let Some(submatches) = matches.subcommand_matches("lock") {
		return lock(submatches.clone(), config).unwrap();
	}

	server(matches, config).unwrap()
}

fn lock(_matches: ArgMatches, _config: Config) -> error::Result<()> {
	Ok(())
}

fn server(_matches: ArgMatches, config: Config) -> error::Result<()> {
	let timer  = Timer::spawn(config.timer())?;
	let auth   = Auth::spawn(config.auth())?;
	let server = Server::spawn(config.server())?;
	let locker = Locker::spawn(config)?;

	// XXX: select! is icky, this works around shadowing the outer name
	let l = locker.as_ref();
	let a = auth.as_ref();
	let s = server.as_ref();
	let t = timer.as_ref();

	let mut locked  = false;
	let mut started = false;
	let mut blanked = false;

	loop {
		select! {
			// Locker events.
			event = l.recv() => {
				match event.unwrap() {
					// Try authorization.
					locker::Response::Password(pwd) => {
						auth.authenticate(pwd).unwrap();
					}

					// On system activity.
					locker::Response::Activity => {
						// If the screen is blanked, unblank it.
						if blanked {
							locker.power(true).unwrap();
							blanked = false;
						}

						// If the saver has started but the screen is not locked, unlock
						// it, otherwise just reset the timers.
						if started {
							if !locked {
								started = false;
								locker.stop().unwrap();
							}
						}
						else {
							timer.reset(timer::Event::Blank);
							timer.reset(timer::Event::Idle);
						}
					}
				}
			},

			// Authentication events.
			event = a.recv() => {
				match event.unwrap() {
					auth::Response::Success => {
						info!("authorization: success");

						locked  = false;
						started = false;

						locker.auth(true).unwrap();
						locker.stop().unwrap();
						timer.restart();
					}

					auth::Response::Failure => {
						info!("authorization: failure");

						locker.auth(false).unwrap();
					}
				}
			},

			// DBus events.
			event = s.recv() => {
				match event.unwrap() {
					event => {
						info!("dbus: {:?}", event);
					}
				}
			},

			// Timer events.
			event = t.recv() => {
				match event.unwrap() {
					timer::Response::Report { .. } => (),

					timer::Response::Heartbeat => {
						locker.sanitize().unwrap();
					}

					timer::Response::Start => {
						started = true;
						locker.start().unwrap();
					}

					timer::Response::Lock => {
						locked = true;
						locker.lock().unwrap();
					}

					timer::Response::Blank => {
						locker.power(false).unwrap();
						blanked = true;
					}
				}
			}
		}
	}
}
