#![feature(type_ascription, question_mark, associated_type_defaults, mpsc_select, stmt_expr_attributes)]

#[macro_use]
extern crate log;
extern crate env_logger;

extern crate clap;
extern crate xdg;
extern crate toml;
extern crate rand;

#[cfg(feature = "dbus")]
extern crate dbus;

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

mod timer;
use timer::Timer;

mod server;
use server::Server;

mod locker;
use locker::Locker;

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
	let server = Server::spawn(config.server())?;
	let locker = Locker::spawn(config)?;

	// XXX: select! is icky, this works around shadowing the outer name
	let t = timer.as_ref();
	let s = server.as_ref();
	let l = locker.as_ref();

	loop {
		select! {
			// Timer events.
			event = t.recv() => {
				let event = event.unwrap();

				match event {
					timer::Response::Report { .. } => (),

					// On heartbeat sanitize the windows from all the bad things that can
					// happen with X11.
					timer::Response::Heartbeat => {
						locker.sanitize();
					}

					timer::Response::Start => {
						locker.start();
					}

					timer::Response::Lock => {
						locker.lock();
					}

					timer::Response::Blank => {
						locker.power(false);
					}
				}
			},

			// DBus events.
			event = s.recv() => {
				let event = event.unwrap();

				info!("server: {:?}", event);
			},

			// Locker events.
			event = l.recv() => {
				let event = event.unwrap();

				match event {
					locker::Response::Keyboard(key) => {
						timer.reset(timer::Event::Blank);
						locker.power(true);

						if let locker::Keyboard::Char('q') = key {
							locker.stop();
							//timer.restart();
						}
					}

					// Reset idle timer.
					locker::Response::Activity => {
						timer.reset(timer::Event::Idle);
					}
				}
			}
		}
	}
}
