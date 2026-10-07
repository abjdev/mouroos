import json
import os
import socket
import subprocess
import time

artifact_dir = "/home/atahan/.gemini/antigravity/brain/d0ef7bc6-65c7-462b-9c92-f6e3a4c8768b"
sock_path = "/tmp/qmp-wheel.sock"
log_path = "/tmp/qemu_wheel.log"

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

print("[TEST] Launching QEMU with mouse scroll test...")
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
    time.sleep(0.08)
    try:
        return sock.recv(4096)
    except:
        return None

def save_screen(name):
    ppm = f"/tmp/{name}.ppm"
    png = os.path.join(artifact_dir, f"{name}.png")
    send_qmp("screendump", {"filename": ppm})
    time.sleep(0.3)
    subprocess.run(["magick", ppm, png], check=True)
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

def scroll_wheel(delta_z, count=5):
    for _ in range(count):
        send_qmp("human-monitor-command", {"command-line": f"mouse_move 0 0 {delta_z}"})
        time.sleep(0.08)

print("[TEST] Pressing Enter to enter desktop...")
send_qmp("send-key", {"keys": [{"type": "qcode", "data": "ret"}]})
time.sleep(2.0)

print("[TEST] Step 1: Terminal - running 'help' command...")
type_string("help\n")
time.sleep(1.0)
save_screen("wheel_1_term_help")

print("[TEST] Step 2: Terminal - scrolling UP into command history (mouse wheel up)...")
# In QEMU monitor, mouse_move 0 0 -1 produces upward scroll
scroll_wheel(-1, count=8)
time.sleep(0.5)
save_screen("wheel_2_term_scrolled_history")

print("[TEST] Step 3: Terminal - scrolling DOWN back towards prompt (mouse wheel down)...")
scroll_wheel(1, count=8)
time.sleep(0.5)
save_screen("wheel_3_term_scrolled_prompt")

print("[TEST] Step 4: Launching Mouros Browser...")
type_string("browser\n")
time.sleep(2.0)
save_screen("wheel_4_browser_top")

print("[TEST] Step 5: Mouros Browser - scrolling DOWN document...")
scroll_wheel(1, count=6)
time.sleep(0.5)
save_screen("wheel_5_browser_scrolled_down")

print("[TEST] Step 6: Mouros Browser - scrolling UP document...")
scroll_wheel(-1, count=6)
time.sleep(0.5)
save_screen("wheel_6_browser_scrolled_up")

if os.path.exists(log_path):
    with open(log_path, "r", errors="ignore") as f:
        print("[SERIAL LOG SNAPSHOT]")
        for line in f:
            if "MOUSE" in line:
                print(line.strip())

print("[TEST] Done!")
try:
    proc.terminate()
except:
    pass
