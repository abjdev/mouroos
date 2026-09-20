import socket
import json
import subprocess
import time
import os

sock_path = "/tmp/qmp-apps.sock"
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

# Unminimize Music Player by clicking its taskbar tab:
# Taskbar tabs are at y = 584. Music tab is tab index 2:
# tab_x starts around 94. tab_w is ~80-100.
# Let's open Music Player, Image Viewer, and ELF Runner via Start Menu or clicking
# Or let's click Start Menu at (20, 585):
def mouse_move_to(tx, ty, cur_x=400, cur_y=300):
    dx = tx - cur_x
    dy = ty - cur_y
    send_qmp("input-send-event", {
        "events": [
            {"type": "rel", "data": {"axis": "x", "value": dx}},
            {"type": "rel", "data": {"axis": "y", "value": dy}}
        ]
    })
    time.sleep(0.1)
    return tx, ty

def mouse_click():
    send_qmp("input-send-event", {
        "events": [{"type": "btn", "data": {"down": True, "button": "left"}}]
    })
    time.sleep(0.05)
    send_qmp("input-send-event", {
        "events": [{"type": "btn", "data": {"down": False, "button": "left"}}]
    })
    time.sleep(0.1)

# Click Start Menu at (30, 585)
cx, cy = mouse_move_to(30, 585, 400, 300)
mouse_click()
time.sleep(0.5)

send_qmp("screendump", {"filename": "/tmp/start_menu_open.ppm"})
subprocess.run(["convert", "/tmp/start_menu_open.ppm", "/home/atahan/.gemini/antigravity/brain/d0ef7bc6-65c7-462b-9c92-f6e3a4c8768b/desktop_start_menu.png"])
print("Captured desktop_start_menu.png")

# Click ELF Runner in Start Menu (item index 8, item_h=25):
# menu_y = 600 - 32 - 284 = 284. item 8 y = 284 + 28 + 8*25 + 10 = 522.
cx, cy = mouse_move_to(60, 522, cx, cy)
mouse_click()
time.sleep(0.5)

send_qmp("screendump", {"filename": "/tmp/elf_runner_open.ppm"})
subprocess.run(["convert", "/tmp/elf_runner_open.ppm", "/home/atahan/.gemini/antigravity/brain/d0ef7bc6-65c7-462b-9c92-f6e3a4c8768b/desktop_elf_runner.png"])
print("Captured desktop_elf_runner.png")

# Now click Start Menu again and open Music Player (item 5, y = 284 + 28 + 5*25 + 10 = 447)
cx, cy = mouse_move_to(30, 585, cx, cy)
mouse_click()
time.sleep(0.3)
cx, cy = mouse_move_to(60, 447, cx, cy)
mouse_click()
time.sleep(0.5)

send_qmp("screendump", {"filename": "/tmp/music_player_open.ppm"})
subprocess.run(["convert", "/tmp/music_player_open.ppm", "/home/atahan/.gemini/antigravity/brain/d0ef7bc6-65c7-462b-9c92-f6e3a4c8768b/desktop_music_player.png"])
print("Captured desktop_music_player.png")

proc.terminate()
proc.wait()
print("All app interactions completed successfully!")
