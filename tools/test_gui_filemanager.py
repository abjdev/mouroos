import socket
import json
import subprocess
import time
import os
import sys

sock_path = "/tmp/qmp-gui.sock"
artifact_dir = "/home/atahan/.gemini/antigravity/brain/d0ef7bc6-65c7-462b-9c92-f6e3a4c8768b"

if os.path.exists(sock_path):
    os.remove(sock_path)

log_path = "/tmp/qemu_gui.log"
if os.path.exists(log_path):
    os.remove(log_path)

if not os.path.exists("disk.img"):
    subprocess.run(["qemu-img", "create", "-f", "raw", "disk.img", "32M"], check=True)

cmd = [
    "qemu-system-x86_64",
    "-cdrom", "mouros.iso",
    "-boot", "d",
    "-drive", "format=raw,file=disk.img,index=0,media=disk",
    "-m", "1G",
    "-vga", "std",
    "-serial", f"file:{log_path}",
    "-qmp", f"unix:{sock_path},server,nowait",
    "-display", "none",
    "-no-reboot"
]

proc = subprocess.Popen(cmd)
time.sleep(3.5)

s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.connect(sock_path)
greeting = s.recv(4096)
req = {"execute": "qmp_capabilities"}
s.sendall(json.dumps(req).encode() + b"\n")
s.recv(4096)

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

def save_screen(ppm_path, png_path):
    send_qmp("screendump", {"filename": ppm_path})
    time.sleep(0.2)
    subprocess.run(["convert", ppm_path, png_path], stderr=subprocess.DEVNULL)
    print(f"Captured: {png_path}")

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

print("1. Dismissing welcome dialog...")
send_qmp("send-key", {"keys": [{"type": "qcode", "data": "ret"}]})
time.sleep(1.5)

print("2. Testing Window Maximize & Restore on Terminal window...")
# Terminal window: x: 280, y: 290, width: 505, height: 265
# Maximize button is at: x + width - 38 + 8 = 755, y + 4 + 7 = 301
mouse_click(755, 301)
time.sleep(0.8)
save_screen("/tmp/gui_1_maximized.ppm", f"{artifact_dir}/gui_1_maximized.png")

# Now window is maximized: x: 0, y: 0, width: 800, height: 572
# Restore button is at: x + width - 38 + 8 = 770, y + 4 + 7 = 11
mouse_click(770, 11)
time.sleep(0.8)
save_screen("/tmp/gui_2_restored.ppm", f"{artifact_dir}/gui_2_restored.png")

print("3. Testing Window Snapping (Alt+Left arrow)...")
send_qmp("input-send-event", {"events": [{"type": "key", "data": {"key": {"type": "qcode", "data": "alt"}, "down": True}}]})
time.sleep(0.1)
send_qmp("input-send-event", {"events": [{"type": "key", "data": {"key": {"type": "qcode", "data": "left"}, "down": True}}]})
time.sleep(0.05)
send_qmp("input-send-event", {"events": [{"type": "key", "data": {"key": {"type": "qcode", "data": "left"}, "down": False}}]})
time.sleep(0.1)
send_qmp("input-send-event", {"events": [{"type": "key", "data": {"key": {"type": "qcode", "data": "alt"}, "down": False}}]})
time.sleep(0.8)
save_screen("/tmp/gui_3_snapped_left.ppm", f"{artifact_dir}/gui_3_snapped_left.png")

print("4. Testing Alt+Tab Window Switcher overlay...")
send_qmp("input-send-event", {"events": [{"type": "key", "data": {"key": {"type": "qcode", "data": "alt"}, "down": True}}]})
time.sleep(0.1)
send_qmp("input-send-event", {"events": [{"type": "key", "data": {"key": {"type": "qcode", "data": "tab"}, "down": True}}]})
time.sleep(0.05)
send_qmp("input-send-event", {"events": [{"type": "key", "data": {"key": {"type": "qcode", "data": "tab"}, "down": False}}]})
time.sleep(0.8)
save_screen("/tmp/gui_4_alttab.ppm", f"{artifact_dir}/gui_4_alttab.png")
# Release Alt to switch
send_qmp("input-send-event", {"events": [{"type": "key", "data": {"key": {"type": "qcode", "data": "alt"}, "down": False}}]})
time.sleep(0.8)

print("5. Testing Start Menu...")
calibrate_mouse_to_zero()
# Start button is at (30, 584)
mouse_click(30, 584)
time.sleep(0.8)
save_screen("/tmp/gui_5_start_menu.ppm", f"{artifact_dir}/gui_5_start_menu.png")

print("6. Launching Mouros Explorer from Start Menu...")
# Item 10: Mouros Explorer is at (80, 528)
mouse_click(80, 528)
time.sleep(1.0)
save_screen("/tmp/gui_6_explorer_open.ppm", f"{artifact_dir}/gui_6_explorer_open.png")

print("7. Testing Explorer File Properties modal...")
# Explorer window: x: 120, y: 70, w: 580, h: 380
# Properties button: client_x(122) + 520 = 642, client_y(92) + 12 = 104
mouse_click(642, 104)
time.sleep(0.8)
save_screen("/tmp/gui_7_explorer_properties.ppm", f"{artifact_dir}/gui_7_explorer_properties.png")

# Close properties by clicking inside the modal
mouse_click(400, 260)
time.sleep(0.5)

print("8. Testing New File creation in Explorer...")
# New File button: client_x(122) + 230 = 352, client_y(92) + 12 = 104
mouse_click(352, 104)
time.sleep(0.8)
save_screen("/tmp/gui_8_new_file_created.ppm", f"{artifact_dir}/gui_8_new_file_created.png")

print("9. Testing Quick Access navigation to /bin...")
# Binaries (/bin) in Quick Access panel: client_x(122) + 50 = 172, client_y(92) + 106 = 198
mouse_click(172, 198)
time.sleep(0.8)
save_screen("/tmp/gui_9_explorer_bin.ppm", f"{artifact_dir}/gui_9_explorer_bin.png")

s.close()
proc.terminate()
proc.wait(timeout=5)

print("\n--- Kernel Serial Log ---")
if os.path.exists(log_path):
    with open(log_path, "r", errors="ignore") as f:
        print(f.read()[-1000:])

print("\nGUI & FILE MANAGER VERIFICATION COMPLETE!")
