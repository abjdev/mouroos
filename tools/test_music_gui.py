import socket
import json
import subprocess
import time
import os

sock_path = "/tmp/qmp-gui.sock"
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

print("Starting QEMU...")
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

# 1. Skip welcome screen
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

def save_screen(ppm_path, png_path):
    send_qmp("screendump", {"filename": ppm_path})
    time.sleep(0.1)
    subprocess.run(["convert", ppm_path, png_path], stderr=subprocess.DEVNULL)
    print(f"Captured: {png_path}")

# 2. Click Start Menu button at (30, 584)
print("Opening Start Menu...")
cur_x, cur_y = move_mouse_smooth(30, 584, cur_x, cur_y)
click()
time.sleep(0.4)

# 3. Click Music Player (Item 5) in Start Menu: y = 284 + 28 + 5*25 + 12 = 449
print("Clicking Music Player in Start Menu...")
cur_x, cur_y = move_mouse_smooth(50, 449, cur_x, cur_y)
click()
time.sleep(0.6)

# Music Player window spawns at (180, 100, 380, 300). Client is at cx=182, cy=124.
# 4. Click PLAY button at screen (218, 274)
print("Clicking PLAY on MP3 Player...")
cur_x, cur_y = move_mouse_smooth(218, 274, cur_x, cur_y)
click()

# Let MP3 playback synthesize and 16-band spectrum visualizer animate
print("Synthesizing audio and animating spectrum bars...")
time.sleep(2.5)

save_screen("/tmp/mp3_active.ppm", "/home/atahan/.gemini/antigravity/brain/d0ef7bc6-65c7-462b-9c92-f6e3a4c8768b/desktop_mp3_player_active.png")

# 5. Click NEXT button at screen (357, 274) to switch to Moonlight track
print("Clicking NEXT button to switch to Track 2 (Moonlight Echoes)...")
cur_x, cur_y = move_mouse_smooth(357, 274, cur_x, cur_y)
click()
time.sleep(2.5)

save_screen("/tmp/mp3_moonlight.ppm", "/home/atahan/.gemini/antigravity/brain/d0ef7bc6-65c7-462b-9c92-f6e3a4c8768b/desktop_mp3_moonlight.png")

# 6. Click MODE toggle button at screen (484, 274) to switch to 8-Bit Chiptune mode
print("Switching to 8-Bit Chiptune mode...")
cur_x, cur_y = move_mouse_smooth(484, 274, cur_x, cur_y)
click()
time.sleep(2.0)

save_screen("/tmp/music_chiptune.ppm", "/home/atahan/.gemini/antigravity/brain/d0ef7bc6-65c7-462b-9c92-f6e3a4c8768b/desktop_music_chiptune_mode.png")

proc.terminate()
proc.wait()
print("All Music GUI tests complete!")
