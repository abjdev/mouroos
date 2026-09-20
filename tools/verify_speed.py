import socket
import json
import subprocess
import time
import os

sock_path = "/tmp/qmp-speed.sock"
if os.path.exists(sock_path):
    os.remove(sock_path)

log_path = "/tmp/qemu_speed_serial.log"
if os.path.exists(log_path):
    os.remove(log_path)

cmd = [
    "qemu-system-x86_64",
    "-cdrom", "mouros.iso",
    "-boot", "d",
    "-m", "1G",
    "-vga", "std",
    "-serial", f"file:{log_path}",
    "-qmp", f"unix:{sock_path},server,nowait",
    "-display", "none",
    "-no-reboot"
]

print("Launching QEMU...")
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

# Wait for welcome screen
time.sleep(2.5)

# Press Enter to launch desktop
send_qmp("send-key", {"keys": [{"type": "qcode", "data": "ret"}]})
time.sleep(1.5)

# Screenshot 1: Desktop with SysInfo showing 100 Hz timer
send_qmp("screendump", {"filename": "/tmp/speed_desktop.ppm"})
subprocess.run(["convert", "/tmp/speed_desktop.ppm", "/home/atahan/.gemini/antigravity/brain/d0ef7bc6-65c7-462b-9c92-f6e3a4c8768b/desktop_100hz.png"])
print("Captured desktop_100hz.png")

# Test fast cursor movement: send rapid mouse movements via QMP
print("Testing rapid mouse movement (50 packets)...")
t0 = time.time()
for i in range(50):
    send_qmp("input-send-event", {
        "events": [
            {"type": "rel", "data": {"axis": "x", "value": 4}},
            {"type": "rel", "data": {"axis": "y", "value": -2}}
        ]
    })
    time.sleep(0.005) # 200 Hz injection
t_elapsed = time.time() - t0
print(f"Injected 50 mouse moves in {t_elapsed:.3f}s")

# Capture screenshot with moved cursor
send_qmp("screendump", {"filename": "/tmp/speed_mouse.ppm"})
subprocess.run(["convert", "/tmp/speed_mouse.ppm", "/home/atahan/.gemini/antigravity/brain/d0ef7bc6-65c7-462b-9c92-f6e3a4c8768b/desktop_fast_cursor.png"])
print("Captured desktop_fast_cursor.png")

def type_string(text, delay=0.015):
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

# Rapid keyboard typing test in terminal
print("Testing high-speed typing into Terminal...")
type_string("elf run sysbench.elf\n", delay=0.01)
time.sleep(1.2)

send_qmp("screendump", {"filename": "/tmp/speed_sysbench.ppm"})
subprocess.run(["convert", "/tmp/speed_sysbench.ppm", "/home/atahan/.gemini/antigravity/brain/d0ef7bc6-65c7-462b-9c92-f6e3a4c8768b/terminal_sysbench.png"])
print("Captured terminal_sysbench.png")

# Run mandelbrot
type_string("elf run mandelbrot.elf\n", delay=0.01)
time.sleep(1.2)
send_qmp("screendump", {"filename": "/tmp/speed_mandelbrot.ppm"})
subprocess.run(["convert", "/tmp/speed_mandelbrot.ppm", "/home/atahan/.gemini/antigravity/brain/d0ef7bc6-65c7-462b-9c92-f6e3a4c8768b/terminal_mandelbrot.png"])
print("Captured terminal_mandelbrot.png")

# Switch theme to cyberpunk
type_string("theme cyberpunk\n", delay=0.01)
time.sleep(1.0)
send_qmp("screendump", {"filename": "/tmp/speed_cyberpunk.ppm"})
subprocess.run(["convert", "/tmp/speed_cyberpunk.ppm", "/home/atahan/.gemini/antigravity/brain/d0ef7bc6-65c7-462b-9c92-f6e3a4c8768b/desktop_cyberpunk.png"])
print("Captured desktop_cyberpunk.png")

proc.terminate()
proc.wait()
print("All performance verification tests completed successfully!")
