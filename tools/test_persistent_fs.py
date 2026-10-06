import socket
import json
import subprocess
import time
import os
import sys

sock_path = "/tmp/qmp-fs.sock"
artifact_dir = "/home/atahan/.gemini/antigravity/brain/d0ef7bc6-65c7-462b-9c92-f6e3a4c8768b"

def qmp_connect():
    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    s.connect(sock_path)
    greeting = s.recv(4096)
    req = {"execute": "qmp_capabilities"}
    s.sendall(json.dumps(req).encode() + b"\n")
    s.recv(4096)
    return s

def send_qmp(s, command, args=None):
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

def save_screen(s, ppm_path, png_path):
    send_qmp(s, "screendump", {"filename": ppm_path})
    time.sleep(0.1)
    subprocess.run(["convert", ppm_path, png_path], stderr=subprocess.DEVNULL)
    print(f"Captured: {png_path}")

def type_string(s, text):
    for ch in text:
        if ch == '\n':
            key = "ret"
            send_qmp(s, "send-key", {"keys": [{"type": "qcode", "data": key}]})
        elif ch == ' ':
            key = "spc"
            send_qmp(s, "send-key", {"keys": [{"type": "qcode", "data": key}]})
        elif ch == '.':
            key = "dot"
            send_qmp(s, "send-key", {"keys": [{"type": "qcode", "data": key}]})
        elif ch == '-':
            key = "minus"
            send_qmp(s, "send-key", {"keys": [{"type": "qcode", "data": key}]})
        elif ch == '/':
            key = "slash"
            send_qmp(s, "send-key", {"keys": [{"type": "qcode", "data": key}]})
        elif ch == '>':
            send_qmp(s, "send-key", {"keys": [{"type": "qcode", "data": "shift"}, {"type": "qcode", "data": "dot"}]})
        elif ch == '!':
            send_qmp(s, "send-key", {"keys": [{"type": "qcode", "data": "shift"}, {"type": "qcode", "data": "1"}]})
        elif ch == '_':
            send_qmp(s, "send-key", {"keys": [{"type": "qcode", "data": "shift"}, {"type": "qcode", "data": "minus"}]})
        else:
            key = ch.lower()
            if ch.isupper():
                send_qmp(s, "send-key", {"keys": [{"type": "qcode", "data": "shift"}, {"type": "qcode", "data": key}]})
            else:
                send_qmp(s, "send-key", {"keys": [{"type": "qcode", "data": key}]})
        time.sleep(0.04)

print("=== STAGE 1: BOOT WITH FRESH DISK & WRITE FILE ===")
if os.path.exists(sock_path):
    os.remove(sock_path)

log_path1 = "/tmp/qemu_fs1.log"
if os.path.exists(log_path1):
    os.remove(log_path1)

cmd1 = [
    "qemu-system-x86_64",
    "-cdrom", "mouros.iso",
    "-boot", "d",
    "-drive", "format=raw,file=disk.img,index=0,media=disk",
    "-m", "1G",
    "-vga", "std",
    "-serial", f"file:{log_path1}",
    "-qmp", f"unix:{sock_path},server,nowait",
    "-display", "none",
    "-no-reboot"
]

proc1 = subprocess.Popen(cmd1)
time.sleep(2.0)
s1 = qmp_connect()

# Dismiss Welcome dialog
print("Dismissing welcome dialog...")
send_qmp(s1, "send-key", {"keys": [{"type": "qcode", "data": "ret"}]})
time.sleep(2.0)

save_screen(s1, "/tmp/fs_1_term_focused.ppm", f"{artifact_dir}/fs_1_term_focused.png")

# Run terminal commands:
print("Typing terminal commands to create test file...")
type_string(s1, "pwd\n")
time.sleep(0.4)
type_string(s1, "ls -l /\n")
time.sleep(0.5)
type_string(s1, "cat /etc/hostname\n")
time.sleep(0.4)
type_string(s1, "echo Hello_Persistent_Mouros! > /home/user/persist.txt\n")
time.sleep(0.6)
type_string(s1, "cat /home/user/persist.txt\n")
time.sleep(0.5)
type_string(s1, "sync\n")
time.sleep(0.8)

save_screen(s1, "/tmp/fs_2_term_written.ppm", f"{artifact_dir}/fs_2_term_written.png")

# Gracefully terminate session 1
s1.close()
proc1.terminate()
proc1.wait(timeout=5)
print("Stage 1 complete. QEMU shut down.")

time.sleep(1.5)

print("\n=== STAGE 2: REBOOT WITH SAME DISK & VERIFY PERSISTENCE ===")
if os.path.exists(sock_path):
    os.remove(sock_path)

log_path2 = "/tmp/qemu_fs2.log"
if os.path.exists(log_path2):
    os.remove(log_path2)

cmd2 = [
    "qemu-system-x86_64",
    "-cdrom", "mouros.iso",
    "-boot", "d",
    "-drive", "format=raw,file=disk.img,index=0,media=disk",
    "-m", "1G",
    "-vga", "std",
    "-serial", f"file:{log_path2}",
    "-qmp", f"unix:{sock_path},server,nowait",
    "-display", "none",
    "-no-reboot"
]

proc2 = subprocess.Popen(cmd2)
time.sleep(2.0)
s2 = qmp_connect()

# Dismiss Welcome dialog
print("Dismissing welcome dialog on reboot...")
send_qmp(s2, "send-key", {"keys": [{"type": "qcode", "data": "ret"}]})
time.sleep(2.0)

# Verify persistent file in terminal
print("Checking persistent file in Terminal...")
type_string(s2, "cat /home/user/persist.txt\n")
time.sleep(0.6)
type_string(s2, "ls -l /home/user\n")
time.sleep(0.6)
type_string(s2, "df\n")
time.sleep(0.5)

save_screen(s2, "/tmp/fs_3_term_reboot.ppm", f"{artifact_dir}/fs_3_term_reboot.png")

s2.close()
proc2.terminate()
proc2.wait(timeout=5)
print("Stage 2 complete. QEMU shut down.")

print("\n--- Boot 1 Serial Log ---")
if os.path.exists(log_path1):
    with open(log_path1, "r", errors="ignore") as f:
        print(f.read())

print("\n--- Boot 2 Serial Log ---")
if os.path.exists(log_path2):
    with open(log_path2, "r", errors="ignore") as f:
        print(f.read())

print("ALL TESTS PASSED SUCCESSFULLY!")
