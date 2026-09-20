import socket
import json
import subprocess
import time
import os

sock_path = "/tmp/qmp-custom.sock"
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

print("Starting QEMU with 7.4 MB ISO (including custom song)...")
proc = subprocess.Popen(cmd)
time.sleep(1.8)

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

# Skip welcome screen
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
    time.sleep(0.04)
    send_qmp("input-send-event", {
        "events": [{"type": "btn", "data": {"down": False, "button": "left"}}]
    })
    time.sleep(0.08)

def type_string(text, delay=0.03):
    for ch in text:
        key = "ret" if ch == "\n" else ("spc" if ch == " " else ch.lower())
        send_qmp("send-key", {"keys": [{"type": "qcode", "data": key}]})
        time.sleep(delay)

def save_screen(ppm_path, png_path):
    send_qmp("screendump", {"filename": ppm_path})
    time.sleep(0.1)
    subprocess.run(["convert", ppm_path, png_path], stderr=subprocess.DEVNULL)
    print(f"Captured: {png_path}")

# 1. Test terminal mp3 list
print("Testing 'mp3 list' in terminal...")
type_string("mp3 list\n")
time.sleep(0.5)

print("Testing 'mp3 info 1' (manifest - Toz Pembe)...")
type_string("mp3 info 1\n")
time.sleep(0.5)

save_screen("/tmp/terminal_custom_mp3.ppm", "/home/atahan/.gemini/antigravity/brain/d0ef7bc6-65c7-462b-9c92-f6e3a4c8768b/terminal_custom_mp3.png")

# 2. Open Start Menu and Launch Music Player
print("Opening Start Menu...")
cur_x, cur_y = move_mouse_smooth(30, 584, cur_x, cur_y)
click()
time.sleep(0.4)

print("Launching Music Player...")
cur_x, cur_y = move_mouse_smooth(50, 449, cur_x, cur_y)
click()
time.sleep(0.6)

# 3. Click PLAY button on Music Player
print("Clicking PLAY on custom song...")
cur_x, cur_y = move_mouse_smooth(218, 274, cur_x, cur_y)
click()

time.sleep(3.0)
save_screen("/tmp/desktop_custom_song_playing.ppm", "/home/atahan/.gemini/antigravity/brain/d0ef7bc6-65c7-462b-9c92-f6e3a4c8768b/desktop_custom_song_playing.png")

proc.terminate()
proc.wait()
print("Custom song verification complete!")
