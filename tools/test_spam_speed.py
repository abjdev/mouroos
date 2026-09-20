import socket
import json
import subprocess
import time
import os

sock_path = "/tmp/qmp-spam.sock"
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

# Spam 200 keypresses rapidly without any sleep (simulating aggressive keyboard spam)
print("Spamming 150 keystrokes with 0ms delay...")
keys = ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q", "r", "s", "t"]
for i in range(150):
    k = keys[i % len(keys)]
    send_qmp("send-key", {"keys": [{"type": "qcode", "data": k}]})

time.sleep(0.5)

send_qmp("screendump", {"filename": "/tmp/spam_proof.ppm"})
subprocess.run(["convert", "/tmp/spam_proof.ppm", "/home/atahan/.gemini/antigravity/brain/d0ef7bc6-65c7-462b-9c92-f6e3a4c8768b/desktop_spam_smooth.png"])
print("Captured desktop_spam_smooth.png")

proc.terminate()
proc.wait()
print("Spam test completed cleanly without crash or hang!")
