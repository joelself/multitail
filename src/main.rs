extern crate getopts;
extern crate term;
extern crate notify;
extern crate fsevent;
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::{thread, env, str};
use std::thread::JoinHandle;
use std::fs::File;
use std::io::SeekFrom;
use std::io::prelude::*;
use term::color::*;
use term::Terminal;
use getopts::Options;
use notify::{RecommendedWatcher, Error, Watcher, op};

macro_rules! lock_wr_fl (
  ($con:ident, $fmtstr:expr, $($args:tt)*) => (
	  {
	  	let mut $con = $con.lock().unwrap();
	  	writeln!($con, $fmtstr, $($args)*).unwrap();
	  	let _unused_ret_val = $con.flush();
	  }
  );
  ($con:ident, $fmtstr:expr) => (
	  {
	  	let mut $con = $con.lock().unwrap();
	  	writeln!($con, $fmtstr).unwrap();
	  	let _unused_ret_val = $con.flush();
	  }
  );
);

fn print_usage(program: &str, opts: Options) {
	let brief = format!("Usage: {} FILE [FILE]*", program);
	print!("{}", opts.usage(&brief));
}

fn main() {
	let args: Vec<String> = env::args().collect();
	let program = args[0].clone();
	let mut opts = Options::new();
	opts.optflag("h", "help", "Print this help menu");
	let matches = match opts.parse(&args[1..]) {
		Ok(m)		=> { m },
		Err(f) 	=> { panic!(f.to_string()) },
	};
	if matches.opt_present("h") {
		print_usage(&program, opts);
		return;
	}

	if !matches.free.is_empty() {
		start_all_tails(matches.free);
	} else {
		print_usage(&program, opts);
		return;
	};
}

fn start_all_tails(matches: Vec<String>) {
	static FG_COLORS: [u16; 8] = [BRIGHT_BLUE, YELLOW, BRIGHT_GREEN, BRIGHT_CYAN, BRIGHT_MAGENTA,
		BRIGHT_WHITE, BRIGHT_CYAN, BRIGHT_RED];
	static BG_COLORS: [u16; 3] = [BLACK, BLUE, RED];
	static FG_COLOR_LEN: usize = 8;
	static TOTAL_COLORS: usize = 24; // FG_COLORS.len() * BG_COLORS.len(), add more BG colors and attributes
	let mut color_idx = 0;
	let mut terminal = term::stdout().unwrap();
	let console = Arc::new(Mutex::new(terminal));
	let mut handles = vec![];
	for filepath in matches.iter() {
		lock_wr_fl!(console, "Starting tail for file: \"{}\".", filepath);
		let console = console.clone();
		let filepath = filepath.clone();
		handles.push(thread::spawn(move || {
			start_tail(filepath, console, FG_COLORS[color_idx % FG_COLOR_LEN].clone(),
				BG_COLORS[color_idx / FG_COLOR_LEN].clone());
			color_idx = (color_idx + 1) % TOTAL_COLORS;
		}));
		while handles.len() > 0 {
			let handle = handles.pop();
			if let Some(h) = handle {
				h.join();
			}
		}
	}
}
struct CrossPlatformChannel {
	join_handle: Option<JoinHandle<()>>,
	watcher: Option<RecommendedWatcher>,
}

impl CrossPlatformChannel {
	// #[cfg(target_os = "macos")]
	// pub fn new<T: Terminal + ?Sized>(tx: Sender<fsevent::Event>, filepath: String,
	// 																	console: &Arc<Mutex<Box<T>>>) -> CrossPlatformChannel {
	// 	let fp = filepath.clone();
	// 	let jh: JoinHandle<()> = thread::spawn(move || {
	//     let fsevent = fsevent::FsEvent::new(tx);
	//     fsevent.append_path(&filepath);
	//     fsevent.observe();
	//   });
	// 	lock_wr_fl!(console, "Got observer for file: \"{}\"", fp);
	// 	CrossPlatformChannel{join_handle: Some(jh), watcher: None}
	// }

	#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
	pub fn new<T: Terminal + ?Sized>(tx: Sender<notify::Event>, filepath: String,
																		console: &Arc<Mutex<Box<T>>>) -> CrossPlatformChannel {
		let w: Result<RecommendedWatcher, Error> = Watcher::new(tx);
		let watcher = match w {
			Ok(mut watcher) => {
				lock_wr_fl!(console, "Got watcher for file: \"{}\"", filepath);
				Some(watcher)
			},
			Err(err) 				=> {
				lock_wr_fl!(console, "Failed to get watcher for file: \"{}\" with error:\n{:?}", filepath,
									err);
				None
			},
		};
		CrossPlatformChannel{join_handle: None, watcher: watcher}
	}
}

fn start_tail<T: Terminal + ?Sized>(filepath: String, console: Arc<Mutex<Box<T>>>, fg_color: u16,
	bg_color: u16) {
	let mut file = open_and_seek(&filepath);
	let (tx, rx) = channel();
	// Currently the notify library for Rust doesn't work with MacOS X FSEvents on Rust 1.6.0,
	// and MacOS 10.10.5, so there's two different config methods for setting up a channel
	let channel = CrossPlatformChannel::new(tx, filepath.clone(), &console);
	loop {

		lock_wr_fl!(console, "Waiting for transmission...");
		match rx.recv() {
			Ok(event) => {
				lock_wr_fl!(console, "Got an event:\n{:?}", event);
				// if event.op.unwrap() == op::WRITE {
				// 	{
				// 		let mut console = console.lock().unwrap();
				// 		writeln!(console, "Got Write Event!");
				// 		console.reset().unwrap();
				// 	}
				// 	//Read from fileß	
				// 	let mut buf: Vec<u8> = vec![];
				// 	let _bytes_read = file.read_to_end(& mut buf).unwrap();
				// 	let last_nl = find_last_nl(&buf);
				// 	{
				// 		// Lock the console, change the color, and write all data
				// 		let mut console = console.lock().unwrap();
				// 		console.fg(fg_color).unwrap();
				// 		console.bg(bg_color).unwrap();
				// 		// console.attr(attr);
				// 		// TODO: actually handle the result
				// 		write!(console, "{}", str::from_utf8(&buf[..last_nl]).unwrap()).unwrap();
				// 		console.reset().unwrap();
				// 		// lock goes out of scope and unlocks
				// 	}
				// 	// Seek back to just after the last nl
				// 	file.seek(SeekFrom::Current(last_nl as i64 - buf.len() as i64)).unwrap();
				// }
			},
			_ => (),
		}
		lock_wr_fl!(console, "Out of match.");
	}
}

fn open_and_seek(filepath: &str) -> File {
	// TODO: ERROR handling and seek to end
	// WARNING: Doesn't actually seek to end yet.
	// file.seek(SeekFrom::End(0)).unwrap();
	return File::open(filepath).unwrap();
}

fn find_last_nl(buf: &Vec<u8>) -> usize {
	// TODO: implement this, search backwards for \n or \n\r, return index that includes the newline
	return buf.len();
}
