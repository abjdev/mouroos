import socket
import json
import subprocess
import time
import os

sock_path = "/tmp/qmp-iso.sock"
if os.path.exists(sock_path):
    os.remove(sock_path)

log_path = "/tmp/qemu_serial.log"
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

# Initial greeting
greeting = s.recv(4096)
send_qmp("qmp_capabilities")

# Wait for boot & welcome screen
time.sleep(2.5)
send_qmp("screendump", {"filename": "/tmp/iso_welcome.ppm"})
subprocess.run(["convert", "/tmp/iso_welcome.ppm", "/tmp/iso_welcome.png"])
print("Captured /tmp/iso_welcome.png")

# Press Enter to launch desktop
send_qmp("send-key", {"keys": [{"type": "qcode", "data": "ret"}]})
time.sleep(1.5)

send_qmp("screendump", {"filename": "/tmp/iso_desktop.ppm"})
subprocess.run(["convert", "/tmp/iso_desktop.ppm", "/tmp/iso_desktop.png"])
print("Captured /tmp/iso_desktop.png")

# Send command to terminal: 'elf run hello.elf'
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

# Focus terminal: click inside terminal window (it's active, but let's click or type directly)
# Terminal window was spawned at x=280, y=290
time.sleep(0.5)
type_string("elf run hello.elf\n")
time.sleep(0.5)
type_string("elf run fibonacci.elf\n")
time.sleep(0.5)
type_string("elf run mandelbrot.elf\n")
time.sleep(0.8)

send_qmp("screendump", {"filename": "/tmp/iso_terminal_elf.ppm"})
subprocess.run(["convert", "/tmp/iso_terminal_elf.ppm", "/tmp/iso_terminal_elf.png"])
print("Captured /tmp/iso_terminal_elf.png")

# Switch theme to Cyberpunk via terminal
type_string("theme cyberpunk\n")
time.sleep(1.0)
send_qmp("screendump", {"filename": "/tmp/iso_theme_cyberpunk.ppm"})
subprocess.run(["convert", "/tmp/iso_theme_cyberpunk.ppm", "/tmp/iso_theme_cyberpunk.png"])
print("Captured /tmp/iso_theme_cyberpunk.png")

# Click Start Menu: mouse at bottom left (x=20, y=585)
# In absolute coordinates, or send mouse events
# Let's open Start Menu by sending Start button click:
# We have mouse coordinates in mouros. Let's send a click or key.
# Wait, let's close qemu cleanly
proc.terminate()
proc.wait()
print("Test completed successfully!")
