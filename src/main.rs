extern crate byteorder;
#[cfg(windows)]
extern crate named_pipe;

use std::fs::File;
use std::io::{self, Read, Write};
use std::ops::DerefMut;
use std::thread;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::channel;
use byteorder::{ByteOrder, NativeEndian, WriteBytesExt};

mod proxy_socket;

const BUFFER_SIZE: u32 = 1024 * 16; // 1024 ^ 2 is the maximum

fn valid_length(length: u32) -> bool {
	return length > 0 && length <= BUFFER_SIZE;
}

fn read_stdin() -> Option<Vec<u8>> {
	let stdin = io::stdin();
	let mut buf = vec![0; 4];
	let mut handle = stdin.lock();

	handle.read_exact(&mut buf).unwrap();
	let len = NativeEndian::read_u32(&buf);

	let mut buffer = vec![0; len as usize];

	if let Ok(_) = handle.read_exact(&mut buffer) {
		if valid_length(len) {
			return Some(buffer);
		}
	}
	None
}

fn write_stdout(buf: &[u8]) -> io::Result<()> {
	let stdout = io::stdout();
	let mut out = stdout.lock();

	out.write_u32::<NativeEndian>(buf.len() as u32)?;
	out.write(buf)?;
	out.flush()?;
	Ok(())
}

fn read_socket<T: Read>(socket: &mut T) -> Option<Vec<u8>> {
	let mut buf = vec![0; BUFFER_SIZE as usize];
	if let Ok(len) = socket.read(&mut buf) {
		return Some(Vec::from(&buf[0..len]));
	}
	None
}

fn write_socket<T: Write>(socket: &mut T, data: &[u8]) -> io::Result<()> {
	socket.write(data)?;
	socket.flush()?;
	Ok(())
}

fn main() {
	let mut log = File::create("proxy.log").unwrap();

	let socket = proxy_socket::connect().unwrap();
	let socket_counter = Arc::new(Mutex::new(socket));

	let mut threads = vec![];

	let (stdin_tx, stdin_rx) = channel::<Vec<u8>>();
	let (stdout_tx, stdout_rx) = channel::<Vec<u8>>();
	let (sw_tx, sw_rx) = channel::<u8>();
	let (log_tx, log_rx) = channel::<(&str, Vec<u8>)>();
	let log_tx2 = log_tx.clone();

	// stdin
	threads.push(thread::spawn(move || {
		loop {
			if let Some(data) = read_stdin() {
				log_tx.send(("\nread_stdin:\n", data.clone())).unwrap();
				stdin_tx.send(data).unwrap();
			}
		}
	}));

	// stdout
	threads.push(thread::spawn(move || {
		loop {
			if let Ok(data) = stdout_rx.recv() {
				write_stdout(&data).unwrap();
			}
		}
	}));

	// socket write
	let socket_write = Arc::clone(&socket_counter);
	threads.push(thread::spawn(move || {
		loop {
			if let Ok(data) = stdin_rx.recv() {
				let mut s = socket_write.lock().unwrap();
				if let Ok(_) = write_socket(s.deref_mut(), &data) {
					sw_tx.send(0).unwrap();
				}
			}
		}
	}));

	// socket read
	let socket_read = Arc::clone(&socket_counter);
	threads.push(thread::spawn(move || {
		loop {
			if let Ok(_) = sw_rx.recv() {
				let mut s = socket_read.lock().unwrap();
				if let Some(data) = read_socket(s.deref_mut()) {
					log_tx2.send(("\nread_socket:\n", data.clone())).unwrap();
					stdout_tx.send(data).unwrap();
				}
			}
		}
	}));

	// logging
	threads.push(thread::spawn(move || {
		loop {
			if let Ok(data) = log_rx.recv() {
				log.write_all(data.0.as_bytes()).unwrap();
				log.write_all(&data.1).unwrap();
				log.write_all(b"\n").unwrap();
				log.flush().unwrap();
			}
		}
	}));

	for t in threads {
		t.join().unwrap();
	}
}
