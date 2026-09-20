import socket
import json
import subprocess
import time
import os

sock_path = "/tmp/qmp-doomfull.sock"
if os.path.exists(sock_path):
    os.remove(sock_path)

cmd = [
    "qemu-system-x86_64",
    "-cdrom", "mouros.iso",
    "-boot", "d",
    "-m", "1G",
    "-vga", "std",
    "-qmp", f"unix:{sock_path},server,nowait",
    "-display", "none",
    "-no-reboot"
]

print("Launching QEMU with mouros.iso (1G RAM)...")
proc = subprocess.Popen(cmd)
time.sleep(1.5)

s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.connect(sock_path)

def send_qmp(command, args=None):
    req = {"execute": command}
    if args:
        req["arguments"] = args
    s.sendall(json.dumps(req).encode() + b"\n")
    data = b""
    while True:
        chunk = s.recv(4096)
        data += chunk
        for line in data.split(b"\n"):
            if not line.strip(): continue
            try:
                msg = json.loads(line)
                if "return" in msg or "error" in msg:
                    return msg
            except:
                pass

greeting = s.recv(4096)
send_qmp("qmp_capabilities")
time.sleep(2.0)

artifact_dir = "/home/atahan/.gemini/antigravity/brain/d0ef7bc6-65c7-462b-9c92-f6e3a4c8768b"

def save_screen(ppm_path, png_path):
    send_qmp("screendump", {"filename": ppm_path})
    time.sleep(0.1)
    subprocess.run(["convert", ppm_path, png_path], stderr=subprocess.DEVNULL)
    print(f"Captured: {png_path}")

# 1. Dismiss Welcome Screen
print("Entering desktop...")
send_qmp("send-key", {"keys": [{"type": "qcode", "data": "ret"}]})
time.sleep(1.5)

cur_x, cur_y = 400, 300

def move_mouse_smooth(tx, ty, cx, cy):
    while cx != tx or cy != ty:
        dx = max(-10, min(10, tx - cx))
        dy = max(-10, min(10, ty - cy))
        send_qmp("input-send-event", {
            "events": [
                {"type": "rel", "data": {"axis": "x", "value": dx}},
                {"type": "rel", "data": {"axis": "y", "value": dy}}
            ]
        })
        cx += dx
        cy += dy
        time.sleep(0.003)
    time.sleep(0.05)
    return cx, cy

def click():
    send_qmp("input-send-event", {
        "events": [{"type": "btn", "data": {"down": True, "button": "left"}}]
    })
    time.sleep(0.05)
    send_qmp("input-send-event", {
        "events": [{"type": "btn", "data": {"down": False, "button": "left"}}]
    })
    time.sleep(0.1)

def type_string(text):
    for ch in text:
        if ch == '\n':
            key = "ret"
        elif ch == ' ':
            key = "spc"
        elif ch == '.':
            key = "dot"
        elif ch == '-':
            key = "minus"
        else:
            key = ch.lower()
        send_qmp("send-key", {"keys": [{"type": "qcode", "data": key}]})
        time.sleep(0.04)

# Capture initial desktop showing DOOM icon
save_screen("/tmp/doom_desktop.ppm", f"{artifact_dir}/desktop_doom_icon.png")

# 2. Click DOOM desktop icon at (105, 310)
print("Clicking DOOM desktop icon at (105, 310)...")
cur_x, cur_y = move_mouse_smooth(105, 310, cur_x, cur_y)
click()
time.sleep(1.5)

save_screen("/tmp/doom_running.ppm", f"{artifact_dir}/desktop_doom_running.png")

# 3. Play DOOM: walk forward, turn right, fire pistol
print("Playing DOOM: walking forward, turning, shooting...")
for _ in range(8):
    send_qmp("send-key", {"keys": [{"type": "qcode", "data": "w"}]})
    time.sleep(0.06)

for _ in range(6):
    send_qmp("send-key", {"keys": [{"type": "qcode", "data": "d"}]})
    time.sleep(0.06)

# Fire pistol
send_qmp("send-key", {"keys": [{"type": "qcode", "data": "f"}]})
time.sleep(0.2)

save_screen("/tmp/doom_gameplay.ppm", f"{artifact_dir}/desktop_doom_gameplay.png")

# 4. Toggle Help overlay ('h')
print("Toggling Help overlay...")
send_qmp("send-key", {"keys": [{"type": "qcode", "data": "h"}]})
time.sleep(0.5)

save_screen("/tmp/doom_help.ppm", f"{artifact_dir}/desktop_doom_help.png")

# Close help ('h') and switch to E1M2 ('m')
send_qmp("send-key", {"keys": [{"type": "qcode", "data": "h"}]})
time.sleep(0.2)
print("Switching to E1M2...")
send_qmp("send-key", {"keys": [{"type": "qcode", "data": "m"}]})
time.sleep(1.2)

save_screen("/tmp/doom_e1m2.ppm", f"{artifact_dir}/desktop_doom_e1m2.png")

# 5. Minimize DOOM window by clicking minimize button at (78 + 644 - 36, 45 + 12) = (686, 57)
print("Minimizing DOOM window...")
cur_x, cur_y = move_mouse_smooth(686, 57, cur_x, cur_y)
click()
time.sleep(0.5)

# Focus Terminal at (450, 400)
print("Focusing Terminal at (450, 400)...")
cur_x, cur_y = move_mouse_smooth(450, 400, cur_x, cur_y)
click()
time.sleep(0.3)

print("Running terminal doom benchmark...")
type_string("doom info\n")
time.sleep(0.6)
type_string("doom bench E1M1\n")
time.sleep(1.8)

save_screen("/tmp/doom_bench.ppm", f"{artifact_dir}/terminal_doom_bench.png")

# 6. Click Start Menu button at (30, 584)
print("Opening Start Menu at (30, 584)...")
cur_x, cur_y = move_mouse_smooth(30, 584, cur_x, cur_y)
click()
time.sleep(0.6)

save_screen("/tmp/doom_start_menu.ppm", f"{artifact_dir}/desktop_start_menu_doom.png")

proc.terminate()
proc.wait()
print("All DOOM verifications finished successfully!")
