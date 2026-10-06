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

def mouse_right_click(x, y):
    move_mouse_smooth(x, y)
    time.sleep(0.05)
    send_qmp("input-send-event", {
        "events": [{"type": "btn", "data": {"down": True, "button": "right"}}]
    })
    time.sleep(0.04)
    send_qmp("input-send-event", {
        "events": [{"type": "btn", "data": {"down": False, "button": "right"}}]
    })
    time.sleep(0.08)

print("1. Dismissing welcome dialog...")
send_qmp("send-key", {"keys": [{"type": "qcode", "data": "ret"}]})
time.sleep(1.5)

print("2. Testing Start Menu...")
calibrate_mouse_to_zero()
# Start button is at (30, 584)
mouse_click(30, 584)
time.sleep(0.8)
save_screen("/tmp/gui_5_start_menu.ppm", f"{artifact_dir}/gui_5_start_menu.png")

print("3. Launching Mouros Explorer from Start Menu...")
# Item 10: Mouros Explorer is at (80, 528)
mouse_click(80, 528)
time.sleep(1.0)
save_screen("/tmp/gui_6_explorer_open.ppm", f"{artifact_dir}/gui_6_explorer_open.png")

print("4. Testing Right-Click Context Menu on File Entry...")
# Right click on the first file entry (notes.txt) at x: 300, y: 176
mouse_right_click(300, 176)
time.sleep(0.8)
save_screen("/tmp/gui_7_explorer_context_item.ppm", f"{artifact_dir}/gui_7_explorer_context_item.png")

# Dismiss context menu by left-clicking outside
mouse_click(140, 140)
time.sleep(0.4)

print("5. Testing Right-Click Context Menu on Empty Folder Area...")
# Right click on empty space below the file list at x: 320, y: 280
mouse_right_click(320, 280)
time.sleep(0.8)
save_screen("/tmp/gui_8_explorer_context_empty.ppm", f"{artifact_dir}/gui_8_explorer_context_empty.png")

print("6. Creating New File from Context Menu...")
# Context menu is at local_x=198 (screen 320), local_y=188 (screen 280).
# Items: 0: Refresh (y: 283), 1: Sep (y: 303), 2: New File (y: 323).
mouse_click(360, 328)
time.sleep(0.8)
save_screen("/tmp/gui_9_explorer_created_via_rightclick.ppm", f"{artifact_dir}/gui_9_explorer_created_via_rightclick.png")

print("7. Testing Quick Access navigation to /bin...")
# Binaries (/bin) in Quick Access panel: x: 172, y: 198
mouse_click(172, 198)
time.sleep(0.8)
save_screen("/tmp/gui_10_explorer_bin.ppm", f"{artifact_dir}/gui_10_explorer_bin.png")

s.close()
proc.terminate()
proc.wait(timeout=5)

print("\n--- Kernel Serial Log ---")
if os.path.exists(log_path):
    with open(log_path, "r", errors="ignore") as f:
        print(f.read()[-1000:])

print("\nEXPLORER RIGHT-CLICK CONTEXT MENU VERIFICATION COMPLETE!")
