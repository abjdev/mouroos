import socket
import json
import subprocess
import time
import os

sock_path = "/tmp/qmp-smooth.sock"
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
time.sleep(2.5)

# Enter desktop
send_qmp("send-key", {"keys": [{"type": "qcode", "data": "ret"}]})
time.sleep(1.2)

# Initial mouse pos is (400, 300)
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
    return cx, cy

def click():
    send_qmp("input-send-event", {
        "events": [{"type": "btn", "data": {"down": True, "button": "left"}}]
    })
    time.sleep(0.03)
    send_qmp("input-send-event", {
        "events": [{"type": "btn", "data": {"down": False, "button": "left"}}]
    })
    time.sleep(0.05)

# 1. Move to MOUROS Start Button at (30, 584)
print("Moving to Start button...")
cur_x, cur_y = move_mouse_smooth(30, 584, cur_x, cur_y)
click()
time.sleep(0.5)

send_qmp("screendump", {"filename": "/tmp/start_menu_smooth.ppm"})
subprocess.run(["convert", "/tmp/start_menu_smooth.ppm", "/home/atahan/.gemini/antigravity/brain/d0ef7bc6-65c7-462b-9c92-f6e3a4c8768b/desktop_start_menu_opened.png"])
print("Captured desktop_start_menu_opened.png")

# 2. Click on Snake Game in Start Menu:
# Start menu is at x=4..214, y = (600-32-284) = 284..568.
# Item 4 is Snake (y: 284 + 28 + 4*25 + 12 = 424)
cur_x, cur_y = move_mouse_smooth(50, 424, cur_x, cur_y)
click()
time.sleep(0.5)

send_qmp("screendump", {"filename": "/tmp/snake_opened.ppm"})
subprocess.run(["convert", "/tmp/snake_opened.ppm", "/home/atahan/.gemini/antigravity/brain/d0ef7bc6-65c7-462b-9c92-f6e3a4c8768b/desktop_snake_game.png"])
print("Captured desktop_snake_game.png")

proc.terminate()
proc.wait()
print("Smooth mouse and app launch test complete!")
