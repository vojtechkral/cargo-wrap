use std::env;
use std::str;
use std::io::{self, Write};
use std::process::{exit, Command, Output};
use std::fs;

static CARGO_ENV: &'static str = "CARGO";
static ARGS_SEP: &'static str = "::";
static CARGO_DEFAULT_CMD: &'static str = "build";

macro_rules! exit_early {
	($($tts:tt)*) => {{
		write!(&mut io::stderr(), "Error: ").ok();
		writeln!(&mut io::stderr(), $($tts)*).ok();
		exit(-1)
	}}
}

pub fn cargo_wrap<F>(build_cmd: F)
	where F: FnOnce(String, Vec<String>) -> (String, Vec<String>) {
	let mut args = env::args();
	args.nth(1).unwrap_or_else(|| exit_early!("Not invoked by cargo"));
	let args: Vec<String> = args.collect();
	let split: Vec<_> = args.splitn(3, |arg| arg == ARGS_SEP).collect();
	let (args_tool, args_cargo) = match split.len() {
		1 => (vec!(), split[0]),
		_ => (split[0].iter().map(|s| s.clone()).collect(), split[1]),
	};

	let cargo = env::var(CARGO_ENV).unwrap_or(String::from("cargo"));

	let mut command = Command::new(&cargo);
	let output = match args_cargo.len() {
		0 => command.arg(CARGO_DEFAULT_CMD),
		_ => command.args(args_cargo),
	}
		.env("CARGO_WRAP", "1")
		.output().unwrap_or_else(|e| exit_early!("Could not run cargo at {}: {}", cargo, e));

	let target = match output {
			Output { status: st, stdout: ref out, .. } if st.success() => {
				String::from(str::from_utf8(out)
					.unwrap_or_else(|_| exit_early!("Cargo output not valid UTF-8"))
					.trim())
			},
			Output { status: st, stdout: ref out, .. } =>
				exit_early!("Cargo failed: {}\n  output:\n{}", st, str::from_utf8(out).unwrap_or("<invalid UTF-8>")),
	};

	// TODO: waiting for CARGO_WRAP, see https://github.com/rust-lang/cargo/issues/3670
	if target.len() == 0 {
		exit_early!("Cargo failed to report a path to the target binary\n\
			Note: cargo-wrap only works with cargo 0.9001 or newer");   // TODO: update with real version
	}

	match fs::metadata(&target) {
		Ok(ref meta) if meta.is_dir() =>
			exit_early!("Path to target as reported by cargo is a directory: '{}'", target),
		Err(e) => exit_early!("Could not find target as reported by cargo: '{}': {}", target, e),
		_ => {},
	}

	let (tool, args_tool) = build_cmd(target, args_tool);
	let mut command = Command::new(&tool);
	match args_tool.len() {
		0 => &mut command,
		_ => command.args(args_tool),
	}.spawn().unwrap_or_else(|e| exit_early!("Error running '{}': {}", tool, e)).wait().ok();
}
