import os
from pathlib import Path
import socket
import subprocess
import sys

socket_path = "/run/backend/backend.sock"
try:
    os.unlink(socket_path)
except FileNotFoundError:
    pass

server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
server.bind(socket_path)
os.chmod(socket_path, 0o660)
server.listen()

greeting = Path("/srv/backend/greeting.txt").read_text().strip()
redis_cli = sys.argv[1]

while True:
    connection, _ = server.accept()
    with connection:
        request = b""
        while b"\r\n\r\n" not in request:
            chunk = connection.recv(4096)
            if not chunk:
                break
            request += chunk
        try:
            query = subprocess.run(
                [
                    redis_cli,
                    "-s",
                    "/run/redis/redis.sock",
                    "PING",
                ],
                check=True,
                capture_output=True,
                text=True,
            ).stdout.strip()
        except subprocess.CalledProcessError:
            body = b"database not ready\n"
            connection.sendall(
                b"HTTP/1.1 503 Service Unavailable\r\n"
                + f"Content-Length: {len(body)}\r\n".encode()
                + b"Connection: close\r\n\r\n"
                + body
            )
            continue
        body = f"{greeting}{os.environ['SUFFIX']}: {query}\n".encode()
        connection.sendall(
            b"HTTP/1.1 200 OK\r\n"
            + b"Content-Type: text/plain\r\n"
            + f"Content-Length: {len(body)}\r\n".encode()
            + b"Connection: close\r\n\r\n"
            + body
        )
