#![feature(type_ascription, question_mark, associated_type_defaults, mpsc_select)]

extern crate toml;
extern crate clap;
extern crate xdg;

extern crate libc;
extern crate dbus;
extern crate x11;

#[macro_use]
extern crate glium;

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
use saver::Saver;

fn main() {
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

fn lock(matches: ArgMatches, config: Config) -> error::Result<()> {
	Ok(())
}

fn server(matches: ArgMatches, config: Config) -> error::Result<()> {
	let server   = Server::spawn(config.server())?;
	let window   = Window::spawn(config.window())?;
	let renderer = Renderer::spawn(window.instance())?;

	window.send(window::Event::Show);

	loop {
		select! {
			event = server.recv() => {

			},

			event = window.recv() => {

			},

			state = renderer.recv() => {

			}
		}
	}

	window.send(window::Event::Hide);

	Ok(())
}
