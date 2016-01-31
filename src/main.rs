extern crate getopts;
extern crate term;extern crate notify;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::thread;
use std::env;
use std::fs::File;
use std::io::SeekFrom;
use std::io::prelude::*;
use term::color::*;
use term::Terminal;
use getopts::Options;
use notify::{RecommendedWatcher, Error, Watcher};
use notify::op;

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
	let terminal = match term::stdout() {
		Some(t) => t,
		None 		=> panic!("Unable to initialize a terminal."),
	};
	let console = Arc::new(Mutex::new(terminal));
	let mut handles = vec![];
	for filepath in matches.iter() {

		let console = console.clone();
		let filepath = filepath.clone();
		handles.push(thread::spawn(move || {
			start_tail(filepath, console, FG_COLORS[color_idx % FG_COLOR_LEN].clone(),
				BG_COLORS[color_idx / FG_COLOR_LEN].clone());
			color_idx = (color_idx + 1) % TOTAL_COLORS;
		}));
	}

	fn start_tail<T: Terminal + ?Sized>(filepath: String, console: Arc<Mutex<Box<T>>>, fg_color: u16,
		bg_color: u16) {
		let mut file = open_and_seek(&filepath);
		let (tx, rx) = channel();

  	let watcher: Result<RecommendedWatcher, Error> = Watcher::new(tx);
  	match watcher {
	    Ok(mut watcher) => {
	      watcher.watch(filepath).unwrap(); // This returns ()

	      loop {
	     		match rx.recv() {
		        Ok(event) => {
		        	if event.op.unwrap() == op::WRITE { 
			        	//Read from fileß	
			        	let mut buf: Vec<u8> = vec![];
			        	let _bytes_read = file.read_to_end(& mut buf).unwrap();
			        	let last_nl = find_last_nl(&buf);
			        	{
			        		// Lock the console, change the color, and write all data
			        		let mut console = console.lock().unwrap();
			        		console.fg(fg_color).unwrap();
			        		console.bg(bg_color).unwrap();
			        		// console.attr(attr);
			        		console.write_all(&buf[..last_nl]).unwrap(); // TODO: actually handle the result
			        		// lock goes out of scope and unlocks
			        	}
			        	// Seek back to just after the last nl
			        	file.seek(SeekFrom::Current(last_nl as i64 - buf.len() as i64)).unwrap();
			        }
		        },
		        _ => (),
		      }
		    }
	    },
	    // TODO: Better error message
	    Err(_) => println!("Error"),
  	}
	}

	fn open_and_seek(filepath: &str) -> File {
		// TODO: ERROR handling and seek to end
		// file.seek(SeekFrom::End(0)).unwrap();
		return File::open(filepath).unwrap();
	}

	fn find_last_nl(buf: &Vec<u8>) -> usize {
		// TODO: implement this, search backwards for \n or \n\r, return index that includes the newline
		return buf.len();
	}
}
