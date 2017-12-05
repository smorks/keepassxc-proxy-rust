use std::env;
use std::io;

#[cfg(not(windows))]
use std::os::unix::net::UnixStream;

#[cfg(windows)]
use named_pipe::PipeClient;

#[cfg(windows)]
pub fn connect() -> io::Result<PipeClient> {
	let username = env::var("USERNAME").unwrap();
	let pipe_name = format!("\\\\.\\pipe\\keepassxc\\{}\\kpxc_server", username);
	let client = PipeClient::connect(pipe_name)?;
	Ok(client)
}

#[cfg(not(windows))]
pub fn connect() -> io::Result<UnixStream> {
	use std::time::Duration;

	let socket_name = "kpxc_server";
	let socket: String;
	if let Ok(xdg) = env::var("XDG_RUNTIME_DIR") {
		socket = format!("{}/{}", xdg, socket_name);
	} else {
		socket = format!("/tmp/{}", socket_name);
	}
	let s = UnixStream::connect(socket)?;
	let timeout: Option<Duration> = Some(Duration::from_secs(1));
	s.set_read_timeout(timeout)?;
	Ok(s)
}
