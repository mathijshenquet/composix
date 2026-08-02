import http.server
import os
import pathlib
import socket
import subprocess
import sys
import time


def verify_backends(psql: str, redis_cli: str) -> None:
    credential = pathlib.Path(os.environ["CREDENTIALS_DIRECTORY"]) / "db-password"
    password = credential.read_text(encoding="utf-8").strip()
    if not password:
        raise RuntimeError("empty db-password credential")
    environment = os.environ.copy()
    environment["PGPASSWORD"] = password
    subprocess.run(
        [
            psql,
            "-h",
            "/run/postgresql",
            "-p",
            "35432",
            "-U",
            "mastodon",
            "-d",
            "postgres",
            "-Atqc",
            "SELECT 1",
        ],
        check=True,
        env=environment,
        stdout=subprocess.DEVNULL,
    )
    subprocess.run(
        [redis_cli, "-s", "/run/redis/redis.sock", "PING"],
        check=True,
        stdout=subprocess.DEVNULL,
    )


def notify_watchdog() -> None:
    address = os.environ["NOTIFY_SOCKET"]
    if address.startswith("@"):
        address = "\0" + address[1:]
    notifier = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
    notifier.connect(address)
    notifier.sendall(b"WATCHDOG=1")
    notifier.close()


class Handler(http.server.BaseHTTPRequestHandler):
    def do_GET(self) -> None:
        if self.path != "/health":
            self.send_error(404)
            return
        body = b"OK\n"
        self.send_response(200)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, format: str, *args: object) -> None:
        return


psql, redis_cli = sys.argv[1:]
surface = pathlib.Path("/mastodon/public/system")
verify_backends(psql, redis_cli)
(surface / "web-started").write_text("credential-source=CREDENTIALS_DIRECTORY\n", encoding="utf-8")
print("mastodon web delaying readiness", flush=True)
time.sleep(3)
server = http.server.HTTPServer(("127.0.0.1", 33000), Handler)
server.timeout = 1
(surface / "web-ready").write_text("postgres=ok redis=ok\n", encoding="utf-8")
print("mastodon web ready", flush=True)
while True:
    server.handle_request()
    notify_watchdog()
