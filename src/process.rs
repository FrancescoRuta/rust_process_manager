use std::{process::{Command, Stdio, ChildStdin, ChildStderr, ChildStdout, Child}, sync::Arc, time::Duration};

use parking_lot::{RwLock, Mutex};

pub struct Process {
	pid: Arc<RwLock<u32>>,
	stdin: Arc<RwLock<ChildStdin>>,
	stdout: Arc<RwLock<ChildStdout>>,
	stderr: Arc<RwLock<ChildStderr>>,
	restart: Arc<Mutex<bool>>,
	stop_process: Arc<Mutex<bool>>,
}

impl Process {
	
	pub fn run(cmd: &str, args: Vec<String>, restart: bool) -> Self {
		let (child, pid, stdin, stdout, stderr) = Self::run_child_process(cmd, &args);
		
		let pid = Arc::new(RwLock::new(pid));
		let stdin = Arc::new(RwLock::new(stdin));
		let stdout = Arc::new(RwLock::new(stdout));
		let stderr = Arc::new(RwLock::new(stderr));
		let restart = Arc::new(Mutex::new(restart));
		let stop_process = Arc::new(Mutex::new(false));
		
		{
			let restart = Arc::clone(&restart);
			let stop_process = Arc::clone(&stop_process);
			let pid = Arc::clone(&pid);
			let stdin = Arc::clone(&stdin);
			let stdout = Arc::clone(&stdout);
			let stderr = Arc::clone(&stderr);
			let cmd = cmd.to_string();
			std::thread::spawn(move || {
				let mut child = child;
				loop {
					loop {
						match child.try_wait() {
							Ok(None) => {
								std::thread::sleep(Duration::from_millis(500));
								if *stop_process.lock() {
									if let Err(_err) = child.kill() {
										//TODO: logs
									}
								}
							}
							Ok(Some(_exit_status)) => {
								//TODO: logs
								break;
							}
							Err(_err) => {
								//TODO: logs
								break;
							}
						}
					}
					if !*restart.lock() { break; }
					let (new_child, new_pid, new_stdin, new_stdout, new_stderr) = Self::run_child_process(&cmd, &args);
					child = new_child;
					*pid.write() = new_pid;
					*stdin.write() = new_stdin;
					*stdout.write() = new_stdout;
					*stderr.write() = new_stderr;
				}
				*pid.write() = 0;
			});
		}
		Self {
			pid,
			stdin,
			stdout,
			stderr,
			restart,
			stop_process,
		}
	}
	
	fn run_child_process(cmd: &str, args: &Vec<String>) -> (Child, u32, ChildStdin, ChildStdout, ChildStderr) {
		let mut child = Command::new(cmd)
			.args(args)
			.stdin(Stdio::piped())
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.spawn()
			.expect("Failed to start the process");
		
		let pid = child.id();
		let stdin = child.stdin.take().expect("Failed to open process stdin");
		let stdout = child.stdout.take().expect("Failed to open process stdout");
		let stderr = child.stderr.take().expect("Failed to open process stderr");
		
		(child, pid, stdin, stdout, stderr)
	}
	
	pub fn stop(&self) {
		*self.restart.lock() = false;
		*self.stop_process.lock() = true;
	}
	
	pub fn pid(&self) -> u32 {
		*self.pid.read()
	}
	pub fn stdin(&self) -> &RwLock<ChildStdin> {
		&*self.stdin
	}
	pub fn stdout(&self) -> &RwLock<ChildStdout> {
		&*self.stdout
	}
	pub fn stderr(&self) -> &RwLock<ChildStderr> {
		&*self.stderr
	}
	
}

impl Drop for Process {
	
	fn drop(&mut self) {
		self.stop();
	}
	
}