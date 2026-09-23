import socket
import json
import subprocess
import time
import os

sock_path = "/tmp/qmp-retro2.sock"
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
time.sleep(1.0)

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

# 1. Capture welcome screen early
time.sleep(0.1)
send_qmp("screendump", {"filename": "/tmp/retro_welcome.ppm"})
subprocess.run(["convert", "/tmp/retro_welcome.ppm", "/home/atahan/.gemini/antigravity/brain/d0ef7bc6-65c7-462b-9c92-f6e3a4c8768b/retro_welcome.png"])
print("Captured retro_welcome.png")

# 2. Skip welcome screen to desktop
time.sleep(0.5)
send_qmp("send-key", {"keys": [{"type": "qcode", "data": "ret"}]})
time.sleep(1.2)

send_qmp("screendump", {"filename": "/tmp/retro_desktop.ppm"})
subprocess.run(["convert", "/tmp/retro_desktop.ppm", "/home/atahan/.gemini/antigravity/brain/d0ef7bc6-65c7-462b-9c92-f6e3a4c8768b/retro_desktop.png"])
print("Captured retro_desktop.png")

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
    time.sleep(0.08)
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

# 3. Move to Start button (x=35, y=584)
print("Moving to Start button...")
cur_x, cur_y = move_mouse_smooth(35, 584, cur_x, cur_y)
click()
time.sleep(0.4)

send_qmp("screendump", {"filename": "/tmp/retro_start_menu.ppm"})
subprocess.run(["convert", "/tmp/retro_start_menu.ppm", "/home/atahan/.gemini/antigravity/brain/d0ef7bc6-65c7-462b-9c92-f6e3a4c8768b/retro_start_menu.png"])
print("Captured retro_start_menu.png")

# 4. In Start Menu:
# menu_y = 572 - 276 = 296
# Calculator is item 2: iy = 296 + 4 + 2*24 = 348..372 (center ~360), ix = 2 + 28 = 30..210 (center ~80)
print("Clicking Calculator in Start Menu...")
cur_x, cur_y = move_mouse_smooth(80, 360, cur_x, cur_y)
click()
time.sleep(0.6)

send_qmp("screendump", {"filename": "/tmp/retro_calculator.ppm"})
subprocess.run(["convert", "/tmp/retro_calculator.ppm", "/home/atahan/.gemini/antigravity/brain/d0ef7bc6-65c7-462b-9c92-f6e3a4c8768b/retro_calculator.png"])
print("Captured retro_calculator.png")

# 5. Open Start Menu again and click Settings:
# Settings is item 7: iy = 296 + 4 + 7*24 = 468..492 (center ~480)
print("Opening Start Menu again...")
cur_x, cur_y = move_mouse_smooth(35, 584, cur_x, cur_y)
click()
time.sleep(0.4)
print("Clicking Desktop Settings in Start Menu...")
cur_x, cur_y = move_mouse_smooth(80, 480, cur_x, cur_y)
click()
time.sleep(0.6)

send_qmp("screendump", {"filename": "/tmp/retro_settings.ppm"})
subprocess.run(["convert", "/tmp/retro_settings.ppm", "/home/atahan/.gemini/antigravity/brain/d0ef7bc6-65c7-462b-9c92-f6e3a4c8768b/retro_settings.png"])
print("Captured retro_settings.png")

# 6. Click on Calculator buttons to do a math calculation (e.g. 7 * 8 = 56)
# Calculator window is at (300, 140, 220, 280)
# Let's see Calculator layout:
# client area: cx = 300+4 = 304, cy = 140+22 = 162
# buttons are rendered in grid.

proc.terminate()
proc.wait()
print("Done!")
