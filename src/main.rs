#![feature(type_ascription, question_mark, associated_type_defaults, mpsc_select)]

#[macro_use]
extern crate log;
extern crate env_logger;

extern crate clap;
extern crate xdg;

extern crate libc;
extern crate dbus;

extern crate x11;

#[macro_use]
extern crate api;

#[cfg(feature = "saver-laughing-man")]
extern crate laughing_man;

use clap::{ArgMatches, Arg, App, SubCommand};
use api::gl::{Surface};

#[macro_use]
mod util;

mod error;

mod config;
use config::Config;

mod timer;
use timer::Timer;

mod server;
use server::Server;

mod window;
use window::Window;

mod renderer;
use renderer::Renderer;

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
	let timer    = Timer::spawn(config.timer())?;
	let server   = Server::spawn(config.server())?;
	let window   = Window::spawn(config.window())?;
	let renderer = Renderer::spawn(window.instance())?;

	// XXX: select! is icky, this works around shadowing the outer name
	let t = timer.as_ref();
	let s = server.as_ref();
	let w = window.as_ref();
	let r = renderer.as_ref();

	loop {
		select! {
			event = t.recv() => {
				let event = event.unwrap();

				match event {
					// On heartbeat sanitize the window from all the bad things that can
					// happen with X11.
					timer::Response::Heartbeat => {
						window.sanitize();
					}

					// Entry point, we're about to start the screensaver, so initialize
					// the renderer and saver.
					timer::Response::Start => {
						timer.started();
						renderer.initialize(laughing_man::new(config.saver("laughing_man")));
					}

					timer::Response::Lock => {

					}

					timer::Response::Blank => {

					}
				}
			},

			event = s.recv() => {
				let event = event.unwrap();

				info!("server: {:?}", event);
			},

			event = w.recv() => {
				let event = event.unwrap();

				match event {
					window::Response::Keyboard(key) => {
						if let window::Keyboard::Char('q') = key {
							break;
						}
					}

					window::Response::Activity => {
						timer.activity();
					}

					// We received the screenshot, start the renderer and show the window.
					window::Response::Screenshot(image) => {
						window.show();
						renderer.start(image);
					}
				}
			},

			event = r.recv() => {
				let event = event.unwrap();

				match event {
					// The renderer has been initialize, take a screenshot of the current
					// screen.
					renderer::Response::Initialized => {
						window.screenshot();
					}

					renderer::Response::Started => {

					}

					renderer::Response::Stopped => {

					}
				}
			}
		}
	}

	window.hide();

	Ok(())
}
