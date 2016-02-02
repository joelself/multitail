extern crate getopts;
extern crate term;
extern crate notify;
#[cfg(target_os = "macos")]
extern crate fsevent;
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::{thread, env, str};
use std::thread::JoinHandle;
use std::fs;
use std::fs::File;
use std::io::SeekFrom;
use std::io::prelude::*;
use term::color::*;
use term::Terminal;
use getopts::Options;
use notify::{RecommendedWatcher, PollWatcher, Error, Watcher, op};

macro_rules! lock_wr_fl (
	($con:ident , $fmtstr:expr, $($args:tt)*) => (
		{
			let mut $con = $con.lock().unwrap();
			write!($con, $fmtstr, $($args)*).unwrap();
			let _unused_ret_val = $con.flush();
			$con.reset().unwrap();
		}
	);
	($con:ident, $fmtstr:expr) => (
		{
			let mut $con = $con.lock().unwrap();
			write!($con, $fmtstr).unwrap();
			let _unused_ret_val = $con.flush();
			$con.reset().unwrap();
		}
	);
	// We don't reset the colors, the next writer is responsible for setting the correct color
	($con:ident : $fg:ident : $bg:ident, $fmtstr:expr, $($args:tt)*) => (
		{
			let mut $con = $con.lock().unwrap();
			$con.fg($fg).unwrap();
			$con.bg($bg).unwrap();
			write!($con, $fmtstr, $($args)*).unwrap();
			let _unused_ret_val = $con.flush();
			$con.reset().unwrap();
		}
	);
	($con:ident : $fg:ident : $bg:ident, $fmtstr:expr) => (
		{
			let mut $con = $con.lock().unwrap();
			$con.fg($fg).unwrap();
			$con.bg($bg).unwrap();
			write!($con, $fmtstr).unwrap();
			let _unused_ret_val = $con.flush();
			$con.reset().unwrap();
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
		println!("free args: {}", matches.free.len());
		start_all_tails(matches.free);
	} else {
		print_usage(&program, opts);
		return;
	};
}

fn start_all_tails(matches: Vec<String>) {
	static FG_COLORS: [u16; 8] = [YELLOW, BRIGHT_BLUE, BRIGHT_GREEN, BRIGHT_CYAN, BRIGHT_MAGENTA,
	BRIGHT_WHITE, BRIGHT_CYAN, BRIGHT_RED];
	static BG_COLORS: [u16; 3] = [BLACK, BLUE, RED];
	static FG_COLOR_LEN: usize = 8;
	static TOTAL_COLORS: usize = 24; // FG_COLORS.len() * BG_COLORS.len(), add more BG colors and attributes
	let mut color_idx = 0;
	let mut terminal = term::stdout().unwrap();
	let console = Arc::new(Mutex::new(terminal));
	let mut handles = vec![];
	for filepath in matches.iter() {
		lock_wr_fl!(console, "\nFollowing \"{}\".", filepath);
		let console = console.clone();
		let filepath = filepath.clone();
		handles.push(thread::spawn(move || {
			start_tail(filepath, console, FG_COLORS[color_idx % FG_COLOR_LEN].clone(),
				BG_COLORS[color_idx / FG_COLOR_LEN].clone());
		}));
		color_idx = (color_idx + 1) % TOTAL_COLORS;
	}

	while handles.len() > 0 {
		let handle = handles.pop();
		if let Some(h) = handle {
			h.join();
		}
	}
}
struct CrossPlatformChannel {
	join_handle: Option<JoinHandle<()>>,
	watcher: Option<RecommendedWatcher>,
}

impl CrossPlatformChannel {
	#[cfg(target_os = "macos")]
	pub fn new<T: Terminal + ?Sized>(tx: Sender<fsevent::Event>, filepath: String,
		console: &Arc<Mutex<Box<T>>>) -> CrossPlatformChannel {
			// let fp = filepath.clone();
			// let jh: JoinHandle<()> = thread::spawn(move || {
			//    let fsevent = fsevent::FsEvent::new(tx);
			//    fsevent.append_path(&filepath);
			//    fsevent.observe();
			//  });
			// lock_wr_fl!(console, "Got observer for file: \"{}\"", fp);
			// CrossPlatformChannel{join_handle: Some(jh), watcher: None}
			// You can't watch some files (a lot of the files you would want to tail) using FSEvents
			// So I'm just going to default to the polling watcher on MacOS
		let mut w: Result<PollWatcher, Error> = PollWatcher::new(tx);
		let watcher = match w {
			Ok(mut watcher) => {
				lock_wr_fl!(console, "\nGot watcher for file: \"{}\"", filepath);
				Some(watcher)
			},
			Err(err) 				=> {
				lock_wr_fl!(console, "\nFailed to get watcher for file: \"{}\" with error:\n{:?}",
					filepath, err);
				None
			},
		};
		CrossPlatformChannel{join_handle: None, watcher: watcher}
	}

	#[cfg(any(target_os = "linux", target_os = "windows"))]
	pub fn new<T: Terminal + ?Sized>(tx: Sender<notify::Event>, filepath: String,
		console: &Arc<Mutex<Box<T>>>) -> CrossPlatformChannel {
		let mut w: Result<RecommendedWatcher, Error> = RecommendedWatcher::new(tx);
		let watcher = match w {
			Ok(mut watcher) => {
				watcher.watch(&filepath);
				lock_wr_fl!(console, "\nGot watcher for file: \"{}\"", filepath);
				Some(watcher)
			},
			Err(err) 				=> {
				lock_wr_fl!(console, "\nFailed to get watcher for file: \"{}\" with error:\n{:?}",
					filepath, err);
				None
			},
		};
		CrossPlatformChannel{join_handle: None, watcher: watcher}
	}
}

fn start_tail<T: Terminal + ?Sized>(filepath: String, console: Arc<Mutex<Box<T>>>, fg_color: u16,
	bg_color: u16) {
	println!("Start tail fg: {}, bg: {}", fg_color, bg_color);
	// Currently the notify library for Rust doesn't work with MacOS X FSEvents on Rust 1.6.0,
	// and MacOS 10.10.5, so there's two different config methods for setting up a channel
	let (tx, rx) = channel();
	let _channel = CrossPlatformChannel::new(tx, filepath.clone(), &console);
	let mut buf: [u8;2048] = [0; 2048];
	let (mut file, buf_slice) = open_and_seek(&filepath, &mut buf);
	lock_wr_fl!(console: fg_color: bg_color, "\n{}", str::from_utf8(buf_slice).unwrap());
	loop {

		match rx.recv() {
			Ok(event) => {
				if event.op.unwrap() == op::WRITE {
					//Read from file
					let mut buf: Vec<u8> = vec![];
					let _bytes_read = file.read_to_end(& mut buf).unwrap();
					let last_nl = find_last_nl(&buf);
					// console.attr(attr);
					// TODO: actually handle the result
					lock_wr_fl!(console : fg_color : bg_color, "{}",
						str::from_utf8(&buf[..last_nl]).unwrap());
					// Seek back to just after the last nl
					file.seek(SeekFrom::Current(last_nl as i64 - buf.len() as i64 - 1)).unwrap();
				}
			},
			_ => (),
		}
	}
}

fn open_and_seek<'a>(filepath: &str, buf: &'a mut [u8;2048]) -> (File, &'a [u8]) {
	// Output up to the last 2 newlines or 2048 bytes, whichever is less
	const GET_BYTES: u64 = 2048u64;
	let mut file = File::open(filepath).unwrap();
	let mut size: u64 = fs::metadata(filepath).unwrap().len();
	if size > GET_BYTES {
		size = GET_BYTES;
	}
	let mut bytes: Vec<u8> = vec![];
	let nls = 0;
	file.seek(SeekFrom::End(-(size as i64))).unwrap();
	file.read_to_end(&mut bytes);
	for i in 0..bytes.len() - 1 {
		if bytes[size as usize -1 - i] == 0x0A && nls == 1 {
			// Found the second newline, don't include it in the returned slice
			return (file, &buf[..i]);
		}
		buf[i] = bytes[i];
	}
	// Didn't find 2 newlines, just return 2048 bytes of data
	return (file, &buf[..]);
}

fn find_last_nl(buf: &Vec<u8>) -> usize {
	let iter = buf.iter().rev();
	let len = buf.len();
	for i in 0..len {
		if buf[len - 1 - i] == 0x0A {
			if i+1 < len && buf[len - 2 - i] == 0x0D {
				return len - 2 - i;
			} else {
				return len - 1 - i;
			}
		}
	}
	return buf.len() - 1;
}
