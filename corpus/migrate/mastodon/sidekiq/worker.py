import os
import pathlib
import socket
import subprocess
import sys


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


psql, redis_cli = sys.argv[1:]
verify_backends(psql, redis_cli)
listener = socket.socket()
listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
listener.bind(("127.0.0.1", 33001))
listener.listen()
listener.settimeout(1)
surface = pathlib.Path("/mastodon/public/system")
(surface / "sidekiq-ready").write_text("postgres=ok redis=ok\n", encoding="utf-8")
print("mastodon sidekiq worker ready", flush=True)
while True:
    try:
        connection, _ = listener.accept()
        connection.close()
    except TimeoutError:
        pass
    notify_watchdog()
