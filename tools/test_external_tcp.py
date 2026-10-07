import json
import os
import socket
import subprocess
import time

sock_path = "/tmp/qmp-tcp.sock"
log_path = "/tmp/qemu_tcp.log"

if os.path.exists(sock_path):
    os.remove(sock_path)
if os.path.exists(log_path):
    os.remove(log_path)

qemu_cmd = [
    "qemu-system-x86_64",
    "-cdrom", "mouros.iso",
    "-boot", "d",
    "-drive", "format=raw,file=disk.img,index=0,media=disk",
    "-m", "1G",
    "-vga", "std",
    "-netdev", "user,id=net0",
    "-device", "rtl8139,netdev=net0",
    "-object", "filter-dump,id=f0,netdev=net0,file=/tmp/net.pcap",
    "-serial", f"file:{log_path}",
    "-qmp", f"unix:{sock_path},server,nowait",
    "-display", "none",
    "-no-reboot"
]

print("[TEST] Launching QEMU...")
proc = subprocess.Popen(qemu_cmd)
time.sleep(3.5)

sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
sock.connect(sock_path)
data = sock.recv(1024)
sock.sendall(b'{"execute": "qmp_capabilities"}\n')
time.sleep(0.5)
data = sock.recv(1024)

def send_qmp(cmd, args=None):
    payload = {"execute": cmd}
    if args:
        payload["arguments"] = args
    sock.sendall(json.dumps(payload).encode() + b"\n")
    time.sleep(0.05)
    try:
        return sock.recv(4096)
    except:
        return None

def send_key(key):
    send_qmp("send-key", {"keys": [{"type": "qcode", "data": key}]})
    time.sleep(0.04)

def send_shift_key(key):
    send_qmp("send-key", {"keys": [{"type": "qcode", "data": "shift"}, {"type": "qcode", "data": key}]})
    time.sleep(0.05)

def type_string(s):
    for ch in s:
        if ch == '\n':
            send_key("ret")
        elif ch == ' ':
            send_key("spc")
        elif ch == '.':
            send_key("dot")
        elif ch == '-':
            send_key("minus")
        elif ch == '/':
            send_key("slash")
        elif ch == ':':
            send_shift_key("semicolon")
        else:
            send_key(ch)
        time.sleep(0.04)

time.sleep(1.0)

# Run curl to the real cloudflare IP
print("[TEST] Running: curl -v http://172.66.147.243/")
type_string("curl -v http://172.66.147.243/\n")
time.sleep(4.0)

# Run dns example.com
print("[TEST] Running: dns example.com")
type_string("dns example.com\n")
time.sleep(3.0)

ppm = "/tmp/external_tcp.ppm"
png = "/tmp/external_tcp.png"
send_qmp("screendump", {"filename": ppm})
time.sleep(0.3)
subprocess.run(["convert", ppm, png], check=True)

proc.terminate()
try:
    proc.wait(timeout=3)
except:
    proc.kill()

print("[TEST] QEMU terminated.")
