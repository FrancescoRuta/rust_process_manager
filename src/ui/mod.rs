use std::{io::{self, Stdout}, sync::{Arc, mpsc::{Receiver, Sender}}, time::Duration};
use crossterm::{terminal::{enable_raw_mode, EnterAlternateScreen, disable_raw_mode, LeaveAlternateScreen}, execute, event::{EnableMouseCapture, DisableMouseCapture}};
use parking_lot::{Mutex, RwLock};
use tui::{backend::CrosstermBackend, Terminal, Frame};
use std::sync::mpsc::{self, TryRecvError};

pub mod network;
pub mod process_list;

pub struct UserInterface {
	terminal: Arc<Mutex<Terminal<CrosstermBackend<Stdout>>>>,
	events: Arc<RwLock<Vec<Box<dyn Fn(crossterm::event::Event) + Send + Sync>>>>,
	tx: Sender<()>,
	rendering_function: Arc<dyn Fn(&mut Frame<CrosstermBackend<Stdout>>)>,
}

impl UserInterface {
	
	pub fn new(rendering_function: impl Fn(&mut Frame<CrosstermBackend<Stdout>>) + Send + Sync + 'static) -> Self {
		enable_raw_mode().unwrap();
		let mut stdout = io::stdout();
		execute!(stdout, EnterAlternateScreen, EnableMouseCapture).unwrap();
		let backend = CrosstermBackend::new(stdout);
		let terminal = Arc::new(Mutex::new(Terminal::new(backend).unwrap()));
		let events = Arc::new(RwLock::new(Vec::new()));
		let rendering_function: Arc<dyn Fn(&mut Frame<CrosstermBackend<Stdout>>) + Send + Sync + 'static> = Arc::new(move |f| (rendering_function)(f));
		let (events_clone, rendering_function_clone, terminal_clone) = (Arc::clone(&events), Arc::clone(&rendering_function), Arc::clone(&terminal));
		let (tx, rx) = mpsc::channel();
		std::thread::spawn(move || 
			Self::manage_events(rx, events_clone, rendering_function_clone, terminal_clone)
		);
		Self {
			terminal,
			events,
			tx,
			rendering_function,
		}
	}
	
	pub fn render(&self) {
		self.terminal.lock().draw(|f| (self.rendering_function)(f)).unwrap();
	}
	
	pub fn on_key_event(&self, callback: impl Fn(crossterm::event::KeyEvent) + Send + Sync + 'static) {
		self.events.write().push(Box::new(move |event| if let crossterm::event::Event::Key(event) = event {
			(callback)(event);
		}));
	}
	
	fn manage_events(rx: Receiver<()>, events: Arc<RwLock<Vec<Box<dyn Fn(crossterm::event::Event) + Send + Sync>>>>, render: Arc<dyn Fn(&mut Frame<CrosstermBackend<Stdout>>) + 'static>, terminal: Arc<Mutex<Terminal<CrosstermBackend<Stdout>>>>) {
		loop {
			if let Ok(true) = crossterm::event::poll(Duration::from_millis(100)) {
				let event = crossterm::event::read().unwrap();
				if let crossterm::event::Event::Resize(_, _) = event {
					terminal.lock().draw(|f| (render)(f)).unwrap();
				}
				events.read().iter().for_each(|event_fn| (event_fn)(event));
			}
			match rx.try_recv() {
				Ok(_) | Err(TryRecvError::Disconnected) => break,
				Err(TryRecvError::Empty) => {}
			}
		}
	}
	
}

impl Drop for UserInterface {
	
	fn drop(&mut self) {
		disable_raw_mode().unwrap();
		let terminal = &mut *self.terminal.lock();
		execute!(
			terminal.backend_mut(),
			LeaveAlternateScreen,
			DisableMouseCapture
		).unwrap();
		terminal.show_cursor().unwrap();
		self.tx.send(()).unwrap();
	}
	
}
