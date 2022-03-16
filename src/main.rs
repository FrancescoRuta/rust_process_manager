use std::{sync::Arc, time::Duration, io::{Read, Write}};

use crossterm::event::KeyCode;
use parking_lot::Mutex;
use process::Process;
use ui::{network::NetworkDataStream, UserInterface};

mod ui;
mod process;

fn main() {
	let p = Process::run("./echo_nums.exe", vec![], true);
	while p.pid() > 0 {
		for _ in 0..5 {
			let stdout = &mut *p.stdout().write();
			let mut buf = [0; 1];
			stdout.read(&mut buf).unwrap();
			std::io::stdout().write(&buf).unwrap();
			std::io::stdout().flush().unwrap();
			std::thread::sleep(Duration::from_secs(1));
		}
		p.stop();
		std::thread::sleep(Duration::from_secs(1));
	}
	/*let nds = NetworkDataStream::new();
	let ui = UserInterface::new(move |f| f.render_widget(nds.get_widget(), f.size()));
	let run = Arc::new(Mutex::new(true));
	let run_clone = Arc::clone(&run);
	ui.on_key_event(move |k| {
		if let KeyCode::Char('q') = k.code {
			*run_clone.lock() = false;
		}
	});
	while *run.lock() {
		ui.render();
		std::thread::sleep(Duration::from_secs(1));
	}*/
}