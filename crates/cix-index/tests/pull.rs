use std::{
    env, fs,
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::PathBuf,
    process::Command,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use cix_common::Ref;
use cix_index::{pull, serve, tag, Store};

fn temporary_dir(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = env::temp_dir().join(format!("cix-index-{label}-{}-{nonce}", std::process::id()));
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn serve_and_pull_follow_the_bare_tag_web_contract() {
    let server_state = temporary_dir("server");
    let client_state = temporary_dir("client");
    let source = temporary_dir("source");
    let input = source.join("input");
    fs::create_dir(&input).unwrap();
    fs::write(input.join("message"), "cix integration test").unwrap();
    let store_path = String::from_utf8(
        Command::new("nix")
            .args(["store", "add-path"])
            .arg(&input)
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    let store_path = store_path.trim().to_owned();

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let root_url = format!("localhost:{port}");

    env::set_var("CIX_STATE_DIR", &server_state);
    tag(&store_path, "family/member:v1", None).unwrap();
    thread::spawn(move || {
        serve(&format!("127.0.0.1:{port}"), Vec::new(), true, None).unwrap();
    });
    wait_for_listen(port);

    let browser = get(port, "/", "text/html", "index.example.test");
    assert_eq!(browser.status, 200);
    assert!(browser.content_type().starts_with("text/html"));
    assert_eq!(browser.header("Vary"), Some("Accept"));
    assert!(browser.body.contains("<html"));

    for accept in ["application/vnd.cix+json;version=1", "application/json"] {
        let response = get(port, "/", accept, "index.example.test");
        assert_eq!(response.status, 200);
        assert!(response
            .content_type()
            .starts_with("application/vnd.cix+json;version=1"));
        assert_eq!(response.header("Vary"), Some("Accept"));
        assert_eq!(response.body, r#"{"names":["family/member"]}"#);
    }
    assert!(
        get(port, "/?format=json", "text/html", "index.example.test")
            .content_type()
            .starts_with("application/vnd.cix+json;version=1")
    );
    assert!(get(
        port,
        "/?format=html",
        "application/json",
        "index.example.test"
    )
    .content_type()
    .starts_with("text/html"));

    let name_page = get(
        port,
        "/family/member",
        "text/html",
        "published.example.test",
    );
    assert_eq!(name_page.status, 200);
    assert!(name_page
        .body
        .contains("cix pull published.example.test/family/member:v1"));
    for accept in ["text/html", "application/vnd.cix+json"] {
        let response = get(port, "/missing", accept, "index.example.test");
        assert_eq!(response.status, 404);
        assert_eq!(response.header("Vary"), Some("Accept"));
    }
    assert!(get(port, "/missing", "text/html", "index.example.test")
        .content_type()
        .starts_with("text/html"));
    assert!(get(
        port,
        "/missing",
        "application/vnd.cix+json",
        "index.example.test"
    )
    .content_type()
    .starts_with("application/vnd.cix+json;version=1"));

    let error = tag(
        "/not/a/store/path",
        &format!("{root_url}/family/member:v1"),
        None,
    )
    .unwrap_err();
    assert!(error
        .to_string()
        .contains("qualified names denote remote state; tags are bare"));

    env::set_var("CIX_STATE_DIR", &client_state);
    assert_eq!(
        pull(
            Some(&format!("{root_url}/family/member:v1")),
            Some("local/member:v1"),
        )
        .unwrap(),
        1
    );
    let local = Ref::parse("local/member:v1").unwrap();
    let metadata = Store::open().unwrap().load(&local).unwrap().unwrap();
    assert_eq!(metadata.upstream.as_deref(), Some(root_url.as_str()));
    assert_eq!(
        metadata
            .entry
            .outputs
            .get("x86_64-linux")
            .unwrap()
            .store_path,
        store_path
    );

    let mirror_state = temporary_dir("mirror");
    env::set_var("CIX_STATE_DIR", &mirror_state);
    assert_eq!(
        pull(Some(&format!("{root_url}/family/member:v1")), None).unwrap(),
        1
    );
    let mirror_listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let mirror_port = mirror_listener.local_addr().unwrap().port();
    drop(mirror_listener);
    thread::spawn(move || {
        serve(&format!("127.0.0.1:{mirror_port}"), Vec::new(), false, None).unwrap();
    });
    wait_for_listen(mirror_port);
    let mirror_names = get(
        mirror_port,
        "/?format=json",
        "application/json",
        "mirror.example.test",
    );
    assert_eq!(mirror_names.status, 200);
    assert_eq!(mirror_names.body, r#"{"names":[]}"#);

    fs::remove_dir_all(server_state).unwrap();
    fs::remove_dir_all(client_state).unwrap();
    fs::remove_dir_all(mirror_state).unwrap();
    fs::remove_dir_all(source).unwrap();
}

fn wait_for_listen(port: u16) {
    const TIMEOUT: Duration = Duration::from_secs(5);
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let deadline = Instant::now() + TIMEOUT;

    loop {
        match TcpStream::connect_timeout(&address, Duration::from_millis(100)) {
            Ok(mut stream) => match readiness_request(&mut stream) {
                Ok(()) => return,
                Err(error) if Instant::now() >= deadline => {
                    panic!(
                        "cix serve listened on {address} but did not answer a readiness request within {} seconds: {error}",
                        TIMEOUT.as_secs(),
                    );
                }
                Err(_) => thread::sleep(Duration::from_millis(10)),
            },
            Err(error) if Instant::now() >= deadline => {
                panic!(
                    "cix serve did not listen on {address} within {} seconds: {error}",
                    TIMEOUT.as_secs(),
                );
            }
            Err(_) => thread::sleep(Duration::from_millis(10)),
        }
    }
}

fn readiness_request(stream: &mut TcpStream) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_millis(100)))?;
    stream.write_all(b"GET / HTTP/1.1\r\nHost: readiness.cix.test\r\nConnection: close\r\n\r\n")?;
    let mut response = [0; 1];
    stream.read_exact(&mut response)?;
    Ok(())
}

struct HttpResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
}

impl HttpResponse {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    fn content_type(&self) -> &str {
        self.header("Content-Type").unwrap_or_default()
    }
}

fn get(port: u16, path: &str, accept: &str, host: &str) -> HttpResponse {
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: {host}\r\nAccept: {accept}\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    let (head, body) = response.split_once("\r\n\r\n").unwrap();
    let mut lines = head.lines();
    let status = lines
        .next()
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    let headers = lines
        .filter_map(|line| {
            line.split_once(": ")
                .map(|(key, value)| (key.to_owned(), value.to_owned()))
        })
        .collect();
    HttpResponse {
        status,
        headers,
        body: body.to_owned(),
    }
}
