use clap::Parser;
use ctrlc;
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use serde::Deserialize;
use std::sync::mpsc::channel;
use std::{
	collections::HashMap,
	env, fs,
	process::{Child, Command},
	thread::sleep,
	time::Duration,
};

fn serde_368_false() -> bool {
	false
}

#[derive(Deserialize, Debug)]
struct ConfigCommand {
	run: String,
	#[serde(default = "serde_368_false")]
	daemon: bool,
}

#[derive(Deserialize, Debug)]
struct Config {
	daemon_process: String,
	setup_steps: Vec<ConfigCommand>,
	takedown_steps: Vec<String>,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
	config: String,
}

macro_rules! get_command {
	($command: expr) => {
		Command::new("/bin/sh").arg("-c").arg($command)
	};
}

fn main() {
	let cli = Cli::parse();

	let mut envars: HashMap<String, String> = HashMap::new();

	for (key, value) in env::vars() {
		envars.insert(key, value);
	}

	let config: Config = toml::from_str(&fs::read_to_string(cli.config).unwrap()).unwrap();

	let daemon = get_command!(config.daemon_process).spawn().unwrap();

	let mut children: Vec<Child> = Vec::new();

	config.setup_steps.iter().enumerate().for_each(|(_i, arg)| {
		if !arg.daemon {
			get_command!(arg.run.clone()).status().unwrap();
		} else {
			children.push(get_command!(arg.run.clone()).spawn().unwrap());
			sleep(Duration::from_millis(500))
		}
	});

	let (tx, rx) = channel::<()>();

	ctrlc::set_handler(move || tx.send(()).unwrap()).unwrap();

	rx.recv().unwrap();
	config
		.takedown_steps
		.iter()
		.enumerate()
		.for_each(|(_i, arg)| {
			get_command!(arg).status().unwrap();
		});
	children.iter().enumerate().for_each(|(_i, arg)| {
		signal::kill(Pid::from_raw(arg.id() as i32), Signal::SIGTERM).unwrap();
	});
	signal::kill(Pid::from_raw(daemon.id() as i32), Signal::SIGTERM).unwrap();
}
