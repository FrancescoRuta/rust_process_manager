use std::{time::{Instant, Duration}, collections::{LinkedList, HashMap, HashSet}, sync::Arc};

use parking_lot::Mutex;
use sysinfo::{NetworkData, NetworkExt, SystemExt, RefreshKind};
use tui::{widgets::{Dataset, GraphType, Chart, Block, Axis, Borders, Widget}, symbols, style::{Color, Style}, text::Span};
pub struct NetworkGraphData {
	title: String,
	transmitted: LinkedList<f64>,
	received: LinkedList<f64>,
	last_capture: Instant,
	last_transmitted: u64,
	last_received: u64,
	color_transmitted: Color,
	color_received: Color,
}

impl NetworkGraphData {
	
	pub fn new(title: &str, net_data: &NetworkData) -> Self {
		assert!(title.is_ascii());
		let title = title.as_bytes();
		let mut new_title = String::with_capacity(title.len());
		for &c in title {
			match c {
				b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b' ' | b'(' | b')' => new_title.push(c as char),
				_ => break,
			}
		}
		let title = new_title;
		Self {
			title,
			transmitted: LinkedList::new(),
			received: LinkedList::new(),
			last_capture: Instant::now(),
			last_received: net_data.total_received(),
			last_transmitted: net_data.total_transmitted(),
			color_transmitted: Color::Indexed(rand::random()),
			color_received: Color::Indexed(rand::random()),
		}
	}
	
	fn update(&mut self, net_data: &NetworkData) {
		let time = Instant::now();
		let total_received = net_data.total_received();
		let total_transmitted = net_data.total_transmitted();
		let time_el = (time - self.last_capture).as_millis() as f64 / 1000.0;
		let received = (total_received - self.last_received) as f64 / 128.0 / 1024.0 / time_el;
		let transmitted = (total_transmitted - self.last_transmitted) as f64 / 128.0 / 1024.0 / time_el;
		self.last_capture = time;
		self.last_received = total_received;
		self.last_transmitted = total_transmitted;
		self.received.push_front(received);
		self.transmitted.push_front(transmitted);
		if self.received.len() > 60 {
			self.received.pop_back();
			self.transmitted.pop_back();
		}
	}
	
}

pub struct NetworkDataStream {
	data: Arc<Mutex<HashMap<String, NetworkGraphData>>>,
	stop_running: Arc<Mutex<bool>>,
}

impl NetworkDataStream {
	
	pub fn new() -> Self {
		let data = Arc::new(Mutex::new(HashMap::new()));
		let data_clone = Arc::clone(&data);
		let stop_running = Arc::new(Mutex::new(false));
		let stop_running_clone = Arc::clone(&stop_running);
		std::thread::spawn(move || {
			Self::refresh(stop_running_clone, data_clone);
		});
		NetworkDataStream {
			data,
			stop_running,
		}
	}
	
	pub fn get_widget(&self) -> NetworkDataWidget {
		NetworkDataWidget {
			data: Arc::clone(&self.data),
		}
	}
	
	fn refresh(stop_running: Arc<Mutex<bool>>, data: Arc<Mutex<HashMap<String, NetworkGraphData>>>) {
		let mut sys = sysinfo::System::new_with_specifics(RefreshKind::new().with_networks().with_networks_list());
		loop {
			std::thread::sleep(Duration::from_secs(1));
			sys.refresh_networks();
			let nets = sys.networks().into_iter();
			{
				let data = &mut *data.lock();
				let mut to_be_removed = data.iter().map(|(k, _)| k.clone()).collect::<HashSet<_>>();
				for (title, net) in nets {
					if let Some(ngd) = data.get_mut(title) {
						ngd.update(net);
						to_be_removed.remove(title);
					} else {
						data.insert(title.clone(), NetworkGraphData::new(title, net));
					}
				}
				to_be_removed.iter().for_each(|k| {data.remove(k);});
			}
			if *stop_running.lock() {
				break;
			}
		}
	}
	
}

impl Drop for NetworkDataStream {
	fn drop(&mut self) {
		*self.stop_running.lock() = true;
	}
}

pub struct NetworkDataWidget {
	data: Arc<Mutex<HashMap<String, NetworkGraphData>>>,
}

impl Widget for NetworkDataWidget {
	fn render(self, area: tui::layout::Rect, buf: &mut tui::buffer::Buffer) {
		if area.area() == 0 {
			return;
		}
		let Self { data } = self;
		let data = &*data.lock();
		let mut datasets_data = Vec::with_capacity(data.len());
		let mut max = 0.0f64;
		for (_, net) in data.into_iter() {
			let received = net.received.iter().enumerate().map(|(t, &d)| (60.0 - t as f64, d)).rev().collect::<Vec<_>>();
			let transmitted = net.transmitted.iter().enumerate().map(|(t, &d)| (60.0 - t as f64, d)).rev().collect::<Vec<_>>();
			max = max.max(received.iter().map(|(_, d)| d).copied().fold(f64::NAN, f64::max));
			max = max.max(transmitted.iter().map(|(_, d)| d).copied().fold(f64::NAN, f64::max));
			datasets_data.push((&net.title, net.color_received, net.color_transmitted, received, transmitted));
		}
		
		let mut datasets = Vec::with_capacity(datasets_data.len());
		for (title, color_received, color_transmitted, received, transmitted) in &datasets_data {
			datasets.push(Dataset::default()
				.name(format!("{} (received)", title))
				.marker(symbols::Marker::Braille)
				.graph_type(GraphType::Line)
				.style(Style::default().fg(*color_received))
				.data(received.as_slice())
			);
			datasets.push(Dataset::default()
				.name(format!("{} (transmitted)", title))
				.marker(symbols::Marker::Braille)
				.graph_type(GraphType::Line)
				.style(Style::default().fg(*color_transmitted))
				.data(transmitted.as_slice())
			);
		}
		
		let (max, um) = if max <= 1.2 {
			(max * 1024.0, "Kbps")
		} else {
			(max, "Mbps")
		};
		
		let max = (max * 1.2).ceil();
		let range = [0.0, max];
		let range_strs = ["0.0".to_string(), format!("{:.1}", max / 2.0), format!("{:.1}", max)];
		
		let chart = Chart::new(datasets)
			.block(Block::default().title("Network").borders(Borders::ALL))
			.x_axis(Axis::default()
				.title(Span::styled("", Style::default()))
				.style(Style::default())
				.bounds([0.0, 60.0]))
			.y_axis(Axis::default()
				.title(Span::styled(um, Style::default()))
				.style(Style::default())
				.bounds(range)
				.labels(range_strs.into_iter().map(Span::from).collect()));
		chart.render(area, buf)
	}
}
