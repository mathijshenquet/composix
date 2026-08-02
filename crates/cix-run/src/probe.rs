use std::ffi::OsStr;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::os::unix::ffi::OsStrExt;
use std::time::Duration;
use std::{env, process, thread};

use anyhow::{bail, Context, Result};
use clap::{Subcommand, ValueEnum};

use crate::spec::parse_duration;

const ATTEMPT_TIMEOUT: Duration = Duration::from_secs(1);
const AWAIT_INTERVAL: Duration = Duration::from_millis(250);

#[derive(Subcommand)]
pub enum Command {
    /// Block until an HTTP or TCP probe first succeeds.
    Await {
        #[arg(value_enum)]
        probe_type: ProbeType,
        target: String,
    },
    /// Leave a resident cgroup pinger that feeds systemd's service watchdog.
    Pinger {
        #[arg(value_enum)]
        probe_type: ProbeType,
        target: String,
        #[arg(long, value_name = "DURATION")]
        every: String,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ProbeType {
    Http,
    Tcp,
}

impl Command {
    pub fn run(self) -> Result<()> {
        match self {
            Self::Await { probe_type, target } => await_success(probe_type, &target),
            Self::Pinger {
                probe_type,
                target,
                every,
            } => start_pinger(probe_type, target, parse_duration(&every)?),
        }
    }
}

fn await_success(probe_type: ProbeType, target: &str) -> Result<()> {
    let notify_socket = env::var_os("NOTIFY_SOCKET");
    let mut failures = 0_u64;
    loop {
        if let Some(socket) = &notify_socket {
            notify_watchdog(socket)?;
        }
        match probe_once(probe_type, target) {
            Ok(()) => return Ok(()),
            Err(error) => {
                if failures.is_multiple_of(4) {
                    eprintln!("readiness probe failed: {error:#}");
                }
                failures += 1;
                thread::sleep(AWAIT_INTERVAL);
            }
        }
    }
}

fn start_pinger(probe_type: ProbeType, target: String, interval: Duration) -> Result<()> {
    let notify_socket =
        env::var_os("NOTIFY_SOCKET").context("liveness pinger requires systemd's NOTIFY_SOCKET")?;
    let result = unsafe { libc::fork() };
    if result < 0 {
        return Err(std::io::Error::last_os_error()).context("forking liveness pinger");
    }
    if result > 0 {
        return Ok(());
    }

    if let Err(error) = pinger_loop(probe_type, &target, interval, &notify_socket) {
        eprintln!("liveness pinger stopped: {error:#}");
        process::exit(1);
    }
    unreachable!("liveness pinger loop does not return successfully")
}

fn pinger_loop(
    probe_type: ProbeType,
    target: &str,
    interval: Duration,
    notify_socket: &OsStr,
) -> Result<()> {
    loop {
        match probe_once(probe_type, target) {
            Ok(()) => {
                notify_watchdog(notify_socket)?;
            }
            Err(error) => {
                eprintln!("liveness watchdog missed: {error:#}");
            }
        }
        thread::sleep(interval);
    }
}

fn probe_once(probe_type: ProbeType, target: &str) -> Result<()> {
    match probe_type {
        ProbeType::Http => probe_http(target),
        ProbeType::Tcp => probe_tcp(target),
    }
}

fn probe_tcp(target: &str) -> Result<()> {
    let authority = normalize_authority(target)?;
    connect(&authority).map(|_| ())
}

fn probe_http(target: &str) -> Result<()> {
    let (authority, path) = target
        .split_once('/')
        .with_context(|| format!("HTTP probe target {target:?} has no request path"))?;
    let address = normalize_authority(authority)?;
    let mut stream = connect(&address)?;
    stream.set_read_timeout(Some(ATTEMPT_TIMEOUT))?;
    stream.set_write_timeout(Some(ATTEMPT_TIMEOUT))?;
    let host = authority
        .strip_prefix(':')
        .map_or(authority, |_| "localhost");
    write!(
        stream,
        "GET /{path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n"
    )?;
    let mut status_line = String::new();
    BufReader::new(stream)
        .read_line(&mut status_line)
        .context("reading HTTP probe response status")?;
    if status_line.is_empty() {
        bail!("HTTP probe returned an empty response");
    }
    let status = status_line
        .split_whitespace()
        .nth(1)
        .context("HTTP probe response has no status code")?
        .parse::<u16>()
        .context("HTTP probe response has an invalid status code")?;
    if !(200..400).contains(&status) {
        bail!("HTTP probe returned status {status}");
    }
    Ok(())
}

fn normalize_authority(target: &str) -> Result<String> {
    if target.contains(['\0', '\n', '\r', ' ', '/']) {
        bail!("probe target {target:?} must be host:port");
    }
    if let Some(port) = target.strip_prefix(':') {
        if port.is_empty() {
            bail!("probe target has no port");
        }
        return Ok(format!("127.0.0.1:{port}"));
    }
    if !target.contains(':') {
        bail!("probe target {target:?} must be host:port");
    }
    Ok(target.to_owned())
}

fn connect(target: &str) -> Result<TcpStream> {
    let addresses = target
        .to_socket_addrs()
        .with_context(|| format!("resolving probe target {target:?}"))?;
    let mut last_error = None;
    for address in addresses {
        match TcpStream::connect_timeout(&address, ATTEMPT_TIMEOUT) {
            Ok(stream) => return Ok(stream),
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error
        .map(anyhow::Error::from)
        .unwrap_or_else(|| anyhow::anyhow!("probe target {target:?} resolves to no addresses")))
    .with_context(|| format!("connecting to probe target {target:?}"))
}

fn notify_watchdog(socket: &OsStr) -> Result<()> {
    let path = socket.as_bytes();
    let abstract_socket = path.first() == Some(&b'@');
    let address_bytes = if abstract_socket { &path[1..] } else { path };
    let path_length = address_bytes.len() + usize::from(!abstract_socket);
    if path_length
        > unsafe { std::mem::zeroed::<libc::sockaddr_un>() }
            .sun_path
            .len()
    {
        bail!("NOTIFY_SOCKET path is too long");
    }

    let descriptor =
        unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_DGRAM | libc::SOCK_CLOEXEC, 0) };
    if descriptor < 0 {
        return Err(std::io::Error::last_os_error()).context("opening notify socket");
    }
    let mut address = unsafe { std::mem::zeroed::<libc::sockaddr_un>() };
    address.sun_family = libc::AF_UNIX as libc::sa_family_t;
    let destination = address.sun_path.as_mut_ptr().cast::<u8>();
    let offset = usize::from(abstract_socket);
    unsafe {
        std::ptr::copy_nonoverlapping(
            address_bytes.as_ptr(),
            destination.add(offset),
            address_bytes.len(),
        );
    }
    let address_length = std::mem::offset_of!(libc::sockaddr_un, sun_path) + offset + path_length;
    let sent = unsafe {
        libc::sendto(
            descriptor,
            b"WATCHDOG=1".as_ptr().cast(),
            b"WATCHDOG=1".len(),
            0,
            (&address as *const libc::sockaddr_un).cast(),
            address_length as libc::socklen_t,
        )
    };
    let result = if sent < 0 {
        Err(std::io::Error::last_os_error()).context("sending WATCHDOG=1")
    } else {
        Ok(())
    };
    unsafe {
        libc::close(descriptor);
    }
    result
}

#[cfg(test)]
mod tests {
    use std::io::Read;
    use std::net::TcpListener;
    use std::os::unix::net::UnixDatagram;
    use std::sync::mpsc;

    use super::*;

    #[test]
    fn tcp_probe_reports_open_and_closed_ports() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let target = listener.local_addr().unwrap().to_string();
        probe_once(ProbeType::Tcp, &target).unwrap();
        drop(listener);
        assert!(probe_once(ProbeType::Tcp, &target).is_err());
    }

    #[test]
    fn http_probe_accepts_success_and_rejects_failure_status() {
        for (status, succeeds) in [(204, true), (503, false)] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let server = thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = Vec::new();
                let mut chunk = [0_u8; 256];
                while !request.ends_with(b"\r\n\r\n") {
                    let length = stream.read(&mut chunk).unwrap();
                    assert_ne!(length, 0);
                    request.extend_from_slice(&chunk[..length]);
                }
                assert!(std::str::from_utf8(&request)
                    .unwrap()
                    .starts_with("GET /healthz HTTP/1.1\r\n"));
                write!(
                    stream,
                    "HTTP/1.1 {status} Test\r\nContent-Length: 0\r\n\r\n"
                )
                .unwrap();
            });
            assert_eq!(
                probe_once(ProbeType::Http, &format!(":{}/healthz", address.port())).is_ok(),
                succeeds
            );
            server.join().unwrap();
        }
    }

    #[test]
    fn await_mode_retries_until_the_probe_succeeds() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let target = address.to_string();
        drop(listener);
        let (sender, receiver) = mpsc::channel();
        let waiter =
            thread::spawn(move || sender.send(await_success(ProbeType::Tcp, &target)).unwrap());
        thread::sleep(Duration::from_millis(300));
        let listener = TcpListener::bind(address).unwrap();
        receiver
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .unwrap();
        waiter.join().unwrap();
        drop(listener);
    }

    #[test]
    fn watchdog_notification_supports_filesystem_sockets() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("notify.sock");
        let socket = UnixDatagram::bind(&path).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        notify_watchdog(path.as_os_str()).unwrap();
        let mut message = [0_u8; 32];
        let length = socket.recv(&mut message).unwrap();
        assert_eq!(&message[..length], b"WATCHDOG=1");
    }
}
