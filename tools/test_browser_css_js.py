import json
import os
import socket
import subprocess
import time

artifact_dir = "/home/atahan/.gemini/antigravity/brain/d0ef7bc6-65c7-462b-9c92-f6e3a4c8768b"
sock_path = "/tmp/qmp-cssjs.sock"
log_path = "/tmp/qemu_cssjs.log"

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

print("[TEST] Launching QEMU with CSS & JS test...")
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

current_mouse_x = 512
current_mouse_y = 384

def calibrate_mouse():
    global current_mouse_x, current_mouse_y
    # Use -50 which is safe in int8 (-128..127). 20 * 50 = 1000 pixels.
    for _ in range(20):
        send_qmp("human-monitor-command", {"command-line": "mouse_move -50 -50"})
        time.sleep(0.04)
    current_mouse_x = 0
    current_mouse_y = 0
    time.sleep(0.2)

def move_mouse_to(x, y):
    global current_mouse_x, current_mouse_y
    dx = x - current_mouse_x
    dy = y - current_mouse_y
    step = 50
    while dx != 0 or dy != 0:
        step_x = max(-step, min(step, dx))
        step_y = max(-step, min(step, dy))
        send_qmp("human-monitor-command", {"command-line": f"mouse_move {step_x} {step_y}"})
        dx -= step_x
        dy -= step_y
        time.sleep(0.04)
    current_mouse_x = x
    current_mouse_y = y
    time.sleep(0.1)

def click_mouse(x=None, y=None):
    if x is not None and y is not None:
        move_mouse_to(x, y)
    send_qmp("human-monitor-command", {"command-line": "mouse_button 1"})
    time.sleep(0.12)
    send_qmp("human-monitor-command", {"command-line": "mouse_button 0"})
    time.sleep(0.25)

print("[TEST] Pressing Enter to enter desktop...")
send_qmp("send-key", {"keys": [{"type": "qcode", "data": "ret"}]})
time.sleep(2.0)

print("[TEST] Launching Mouros Browser...")
type_string("browser\n")
time.sleep(2.0)

print("[TEST] Step 1: Capture initial styled browser page (CSS & JS showcase)...")
save_screen("css_js_1_initial")

print("[TEST] Calibrating mouse...")
calibrate_mouse()

print("[TEST] Step 2: Clicking [+] button to increment counter via JavaScript...")
# Button [+] is at (280, 365)
click_mouse(280, 365)
time.sleep(0.5)

print("[TEST] Clicking [+] button a second time...")
click_mouse(280, 365)
time.sleep(0.5)
save_screen("css_js_2_after_click_increment")

print("[TEST] Step 3: Clicking [Toggle Card Color] button (JavaScript DOM style mutation)...")
# [Toggle Card Color] is at (218, 403)
click_mouse(218, 403)
time.sleep(0.5)
save_screen("css_js_3_color_changed")

print("[TEST] Step 4: Clicking [Trigger Alert] button (JavaScript alert dialog)...")
# [Trigger Alert] is at (392, 409)
click_mouse(392, 409)
time.sleep(0.5)
save_screen("css_js_4_alert_shown")

print("[TEST] Step 5: Dismissing JavaScript Alert dialog by clicking [OK] button...")
# Centered alert dialog OK button is around (400, 305)
click_mouse(400, 305)
time.sleep(0.5)
save_screen("css_js_5_alert_dismissed")

if os.path.exists(log_path):
    with open(log_path, "r", errors="ignore") as f:
        print("[SERIAL LOG SNAPSHOT]")
        for line in f:
            if "JS" in line or "alert" in line or "MOUSE" in line:
                print(line.strip())

print("[TEST] All tests completed successfully!")
try:
    proc.terminate()
except:
    pass
