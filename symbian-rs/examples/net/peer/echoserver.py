"""A deterministic, offline TCP server for examples/net: reads a line, writes it back
uppercased with a fixed suffix, then closes. One connection at a time, no dependencies."""
import socket, sys, threading, time

PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 18974
HOST = sys.argv[2] if len(sys.argv) > 2 else "127.0.0.1"

srv = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
srv.bind((HOST, PORT))
srv.listen(8)
print(f"listening on {HOST}:{PORT}", flush=True)
while True:
    conn, peer = srv.accept()
    print(f"connection from {peer}", flush=True)
    try:
        data = conn.recv(1024)
        print(f"  got {data!r}", flush=True)
        conn.sendall(data.strip().upper() + b"-PONG\n")
    except Exception as e:
        print(f"  error {e}", flush=True)
    finally:
        conn.close()
