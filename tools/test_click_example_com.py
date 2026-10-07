import json
import os
import socket
import subprocess
import time

artifact_dir = "/home/atahan/.gemini/antigravity/brain/d0ef7bc6-65c7-462b-9c92-f6e3a4c8768b"
sock_path = "/tmp/qmp-example.sock"
log_path = "/tmp/qemu_example.log"

if os.path.exists(sock_path):
    os.remove(sock_path)
if os.path.exists(log_path):
    os.remove(log_path)

if not os.path.exists("disk.img"):
    subprocess.run(["qemu-img", "create", "-f", "raw", "disk.img", "32M"], check=True)

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

print("[TEST] Launching QEMU...")
proc = subprocess.Popen(qemu_cmd)
time.sleep(3.5)

# Connect QMP
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

def send_key(key):
    send_qmp("send-key", {"keys": [{"type": "qcode", "data": key}]})
    time.sleep(0.04)

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
        else:
            send_key(ch)
        time.sleep(0.04)

cur_x, cur_y = 400, 300

def calibrate_mouse_to_zero():
    global cur_x, cur_y
    for _ in range(60):
        send_qmp("input-send-event", {
            "events": [
                {"type": "rel", "data": {"axis": "x", "value": -16}},
                {"type": "rel", "data": {"axis": "y", "value": -16}}
            ]
        })
        time.sleep(0.003)
    cur_x, cur_y = 0, 0

def move_mouse_smooth(tx, ty):
    global cur_x, cur_y
    while cur_x != tx or cur_y != ty:
        dx = max(-10, min(10, tx - cur_x))
        dy = max(-10, min(10, ty - cur_y))
        send_qmp("input-send-event", {
            "events": [
                {"type": "rel", "data": {"axis": "x", "value": dx}},
                {"type": "rel", "data": {"axis": "y", "value": dy}}
            ]
        })
        cur_x += dx
        cur_y += dy
        time.sleep(0.003)

def mouse_click(x, y):
    move_mouse_smooth(x, y)
    time.sleep(0.05)
    send_qmp("input-send-event", {
        "events": [{"type": "btn", "data": {"down": True, "button": "left"}}]
    })
    time.sleep(0.04)
    send_qmp("input-send-event", {
        "events": [{"type": "btn", "data": {"down": False, "button": "left"}}]
    })
    time.sleep(0.08)

# Wait for desktop ready
time.sleep(1.0)

# Step 1: Launch Browser
print("[TEST] Launching Browser with default welcome page...")
type_string("browser\n")
time.sleep(2.5)

# Step 2: Calibrate and click 'Example Domain (http://example.com/)' at (220, 295)
print("[TEST] Calibrating mouse...")
calibrate_mouse_to_zero()

print("[TEST] Clicking 'Example Domain' link at (220, 295)...")
mouse_click(220, 295)

# Wait for DNS resolution and connection attempt
print("[TEST] Waiting 3.5s for navigation (DNS resolution + TCP connection)...")
time.sleep(3.5)
save_screen("net_11_example_com_link_clicked")

# Step 3: Verify GUI responsiveness by opening Start Menu
print("[TEST] Testing GUI responsiveness: clicking Start Menu at (30, 586)...")
mouse_click(30, 586)
time.sleep(0.8)
save_screen("net_11_gui_responsive")

# Check serial log
with open(log_path, "r", errors="ignore") as f:
    log_content = f.read()
    print("=== SERIAL LOG PREVIEW ===")
    for line in log_content.splitlines()[-35:]:
        print(f"  {line}")
    print("==========================")

proc.terminate()
try:
    proc.wait(timeout=3)
except:
    proc.kill()

print("[TEST] Completed successfully!")
