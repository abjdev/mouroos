import json
import os
import socket
import subprocess
import time

artifact_dir = "/home/atahan/.gemini/antigravity/brain/d0ef7bc6-65c7-462b-9c92-f6e3a4c8768b"
sock_path = "/tmp/qmp-https.sock"
log_path = "/tmp/qemu_https.log"

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
    "-serial", f"file:{log_path}",
    "-qmp", f"unix:{sock_path},server,nowait",
    "-display", "none",
    "-no-reboot"
]

print("[TEST] Launching QEMU with RTL8139...")
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

def save_screen(name):
    ppm = f"/tmp/{name}.ppm"
    png = os.path.join(artifact_dir, f"{name}.png")
    send_qmp("screendump", {"filename": ppm})
    time.sleep(0.3)
    subprocess.run(["convert", ppm, png], check=True)
    print(f"[TEST] Saved screenshot: {png}")

def type_string(s):
    for ch in s:
        if ch == '\n':
            key = "ret"
        elif ch == ' ':
            key = "spc"
        elif ch == '.':
            key = "dot"
        elif ch == '/':
            key = "slash"
        elif ch == ':':
            send_qmp("send-key", {"keys": [{"type": "qcode", "data": "shift"}, {"type": "qcode", "data": "semicolon"}]})
            time.sleep(0.04)
            continue
        elif ch == '-':
            key = "minus"
        elif ch == '_':
            send_qmp("send-key", {"keys": [{"type": "qcode", "data": "shift"}, {"type": "qcode", "data": "minus"}]})
            time.sleep(0.04)
            continue
        else:
            key = ch.lower()
        send_qmp("send-key", {"keys": [{"type": "qcode", "data": key}]})
        time.sleep(0.04)

print("[TEST] Pressing Enter to enter desktop...")
send_qmp("send-key", {"keys": [{"type": "qcode", "data": "ret"}]})
time.sleep(2.0)
save_screen("https_1_desktop_ready")

print("[TEST] Waiting 4 seconds for DHCP...")
time.sleep(4.0)

print("[TEST] Running curl -v https://abjdev.github.io/hw.html in Terminal...")
type_string("curl -v https://abjdev.github.io/hw.html\n")

for i in range(8):
    time.sleep(1.0)

save_screen("https_2_terminal_curl")

print("[TEST] Waiting 2 seconds then launching Mouros Browser with https://abjdev.github.io/hw.html...")
time.sleep(2.0)
type_string("browser https://abjdev.github.io/hw.html\n")

for i in range(10):
    time.sleep(1.0)

save_screen("https_3_browser_hw_rendered")

if os.path.exists(log_path):
    with open(log_path, "r", errors="ignore") as f:
        print("[SERIAL LOG SNAPSHOT]")
        print(f.read()[-3000:])

print("[TEST] Done!")
try:
    proc.terminate()
except:
    pass
