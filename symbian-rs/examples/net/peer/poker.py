"""The other half of examples/net's TcpListener case: keeps trying to connect to the
listener the emulated application opens, sends one line, and prints what came back.
Deterministic and offline; it only ever talks to this host's loopback."""
import socket, sys, time

PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 18975
DEADLINE = time.time() + (float(sys.argv[2]) if len(sys.argv) > 2 else 240.0)

while time.time() < DEADLINE:
    try:
        s = socket.create_connection(("127.0.0.1", PORT), 2)
    except OSError:
        time.sleep(0.5)
        continue
    print("connected to the emulated listener", flush=True)
    try:
        s.sendall(b"poke\n")
        s.shutdown(socket.SHUT_WR)
        print(f"  answer {s.recv(64)!r}", flush=True)
    except Exception as e:
        print(f"  error {e}", flush=True)
    finally:
        s.close()
    break
else:
    print("never connected", flush=True)
