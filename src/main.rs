#![feature(type_ascription, question_mark, associated_type_defaults, mpsc_select, box_syntax)]

#[macro_use]
extern crate log;
extern crate env_logger;

extern crate toml;
extern crate clap;
extern crate xdg;

extern crate libc;
extern crate dbus;

#[macro_use]
extern crate glium;
extern crate image;
extern crate x11;

use clap::{ArgMatches, Arg, App, SubCommand};
use glium::{Surface};

#[macro_use]
mod util;
use util::DurationExt;

mod error;
pub use error::Error;

mod config;
use config::Config;

mod server;
use server::Server;

mod window;
use window::Window;

mod renderer;
use renderer::Renderer;

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

	return server(matches, config).unwrap();
}

fn lock(_matches: ArgMatches, _config: Config) -> error::Result<()> {
	Ok(())
}

fn server(_matches: ArgMatches, config: Config) -> error::Result<()> {
	let server   = Server::spawn(config.server())?;
	let window   = Window::spawn(config.window())?;
	let renderer = Renderer::spawn(window.instance())?;

	// XXX
	window.screenshot();

	// XXX: select! is icky, this works around shadowing the outer name
	let s = server.as_ref();
	let w = window.as_ref();
	let r = renderer.as_ref();

	loop {
		select! {
			event = s.recv() => {
				info!("server: {:?}", event);
			},

			event = w.recv() => {
				match event {
					Ok(window::Event::Keyboard(window::Keyboard::Char('q'))) =>
						break,

					Ok(window::Event::Screen(window::Screen::Response(screen))) => {
						window.show();
						renderer.start(box saver::laughing_man::Saver::new(config.saver("laughing_man"))?, screen);
					}

					_ => ()
				}
			},

			event = r.recv() => {
				match event {
					Ok(renderer::Event::State(state)) => {
						info!("renderer: {:?}", state);
					}

					_ => ()
				}
			}
		}
	}

	window.hide();

	Ok(())
}
