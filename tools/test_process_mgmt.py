import socket
import json
import subprocess
import time
import os
import sys

sock_path = "/tmp/qmp-proc.sock"
artifact_dir = "/home/atahan/.gemini/antigravity/brain/d0ef7bc6-65c7-462b-9c92-f6e3a4c8768b"

if os.path.exists(sock_path):
    os.remove(sock_path)

log_path = "/tmp/qemu_proc.log"
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

def type_string(text):
    for ch in text:
        if ch == '\n':
            key = "ret"
            send_qmp("send-key", {"keys": [{"type": "qcode", "data": key}]})
        elif ch == ' ':
            key = "spc"
            send_qmp("send-key", {"keys": [{"type": "qcode", "data": key}]})
        elif ch == '.':
            key = "dot"
            send_qmp("send-key", {"keys": [{"type": "qcode", "data": key}]})
        elif ch == '-':
            key = "minus"
            send_qmp("send-key", {"keys": [{"type": "qcode", "data": key}]})
        elif ch == '/':
            key = "slash"
            send_qmp("send-key", {"keys": [{"type": "qcode", "data": key}]})
        elif ch == '&':
            send_qmp("send-key", {"keys": [{"type": "qcode", "data": "shift"}, {"type": "qcode", "data": "7"}]})
        else:
            key = ch.lower()
            if ch.isupper():
                send_qmp("send-key", {"keys": [{"type": "qcode", "data": "shift"}, {"type": "qcode", "data": key}]})
            else:
                send_qmp("send-key", {"keys": [{"type": "qcode", "data": key}]})
        time.sleep(0.08)

print("Dismissing welcome dialog...")
send_qmp("send-key", {"keys": [{"type": "qcode", "data": "ret"}]})
time.sleep(1.5)

print("1. Running 'ps' to inspect default system tasks (PIDs 0-4)...")
type_string("ps\n")
time.sleep(1.0)
save_screen("/tmp/proc_1_ps.ppm", f"{artifact_dir}/proc_1_ps.png")

print("2. Running 'top' and 'elf run hello'...")
type_string("top\n")
time.sleep(1.0)
type_string("elf run hello\n")
time.sleep(1.2)
save_screen("/tmp/proc_2_top_and_elf.ppm", f"{artifact_dir}/proc_2_top_and_elf.png")

print("3. Testing kill safety and background task execution...")
type_string("kill 0\n")
time.sleep(0.8)
type_string("elf run fibonacci &\n")
time.sleep(1.0)
type_string("ps\n")
time.sleep(1.0)
save_screen("/tmp/proc_3_kill_and_ps.ppm", f"{artifact_dir}/proc_3_kill_and_ps.png")

s.close()
proc.terminate()
proc.wait(timeout=5)

print("\n--- Kernel Serial Log ---")
if os.path.exists(log_path):
    with open(log_path, "r", errors="ignore") as f:
        print(f.read())

print("PROCESS MANAGEMENT VERIFICATION COMPLETE!")
