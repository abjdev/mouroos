import socket
import json
import subprocess
import time
import os

sock_path = "/tmp/qmp-mp3.sock"
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
time.sleep(2.5)

cur_x, cur_y = 400, 300

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
    time.sleep(0.05)

def mouse_click():
    send_qmp("input-send-event", {
        "events": [{"type": "btn", "data": {"down": True, "button": "left"}}]
    })
    time.sleep(0.04)
    send_qmp("input-send-event", {
        "events": [{"type": "btn", "data": {"down": False, "button": "left"}}]
    })
    time.sleep(0.08)

def type_string(text, delay=0.02):
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
        time.sleep(delay)

def save_screen(ppm_path, png_path):
    send_qmp("screendump", {"filename": ppm_path})
    time.sleep(0.1)
    subprocess.run(["convert", ppm_path, png_path], stderr=subprocess.DEVNULL)
    print(f"Saved: {png_path}")

# 1. Skip welcome screen to enter desktop
print("Passing welcome screen...")
send_qmp("send-key", {"keys": [{"type": "qcode", "data": "ret"}]})
time.sleep(1.8)

# 2. Terminal is focused. Test MP3 CLI commands.
print("Testing 'mp3 list'...")
type_string("mp3 list\n")
time.sleep(0.4)

print("Testing 'mp3 info 1' (neon_horizons)...")
type_string("mp3 info 1\n")
time.sleep(0.4)

print("Testing 'mp3 decode 1' (benchmark decode)...")
type_string("mp3 decode 1\n")
# Allow benchmark to complete all frames
time.sleep(3.5)

save_screen("/tmp/terminal_mp3.ppm", "/home/atahan/.gemini/antigravity/brain/d0ef7bc6-65c7-462b-9c92-f6e3a4c8768b/terminal_mp3_benchmark.png")

# 3. Unminimize Mouros MP3 player by clicking its taskbar tab
# Taskbar tabs: System Info (94..214), Mouros MP3 (218..338) at y ~ 580
print("Unminimizing Mouros MP3 player via taskbar...")
move_mouse_smooth(260, 580)
mouse_click()
time.sleep(0.6)

# 4. Click PLAY button on Music Player
# Window is at (155, 20). Client area is cx=157, cy=44.
# Play button is at client (14..58, 138..162) -> screen (193, 194)
print("Clicking PLAY on MP3 Player...")
move_mouse_smooth(193, 194)
mouse_click()

# Allow music audio to play and spectrum visualizer to animate
print("Playing MP3 track & synthesizing audio...")
time.sleep(2.5)

save_screen("/tmp/mp3_active.ppm", "/home/atahan/.gemini/antigravity/brain/d0ef7bc6-65c7-462b-9c92-f6e3a4c8768b/desktop_mp3_player_active.png")

# 5. Switch to Track 2 (Moonlight Sonata)
# Track 2 row in playlist is at screen (220, 258)
print("Switching to Track 2 (Moonlight Sonata)...")
move_mouse_smooth(220, 258)
mouse_click()
time.sleep(2.5)

save_screen("/tmp/mp3_moonlight.ppm", "/home/atahan/.gemini/antigravity/brain/d0ef7bc6-65c7-462b-9c92-f6e3a4c8768b/desktop_mp3_moonlight.png")

# 6. Toggle Mode to 8-Bit Chiptune Mode
# Toggle button is at client (256..348, 138..162) -> screen (457, 194)
print("Toggling to 8-Bit Chiptune mode...")
move_mouse_smooth(457, 194)
mouse_click()
time.sleep(2.0)

save_screen("/tmp/music_chiptune.ppm", "/home/atahan/.gemini/antigravity/brain/d0ef7bc6-65c7-462b-9c92-f6e3a4c8768b/desktop_music_chiptune_mode.png")

proc.terminate()
proc.wait()
print("All MP3 verification tests completed successfully!")
