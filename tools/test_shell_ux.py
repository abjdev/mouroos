import socket
import json
import subprocess
import time
import os
import sys

sock_path = "/tmp/qmp-shell.sock"
artifact_dir = "/home/atahan/.gemini/antigravity/brain/d0ef7bc6-65c7-462b-9c92-f6e3a4c8768b"

if os.path.exists(sock_path):
    os.remove(sock_path)

log_path = "/tmp/qemu_shell.log"
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
        elif ch == '\t':
            key = "tab"
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
        elif ch == '=':
            key = "equal"
            send_qmp("send-key", {"keys": [{"type": "qcode", "data": key}]})
        elif ch == '|':
            send_qmp("send-key", {"keys": [{"type": "qcode", "data": "shift"}, {"type": "qcode", "data": "backslash"}]})
        elif ch == '$':
            send_qmp("send-key", {"keys": [{"type": "qcode", "data": "shift"}, {"type": "qcode", "data": "4"}]})
        elif ch == '>':
            send_qmp("send-key", {"keys": [{"type": "qcode", "data": "shift"}, {"type": "qcode", "data": "dot"}]})
        elif ch == '<':
            send_qmp("send-key", {"keys": [{"type": "qcode", "data": "shift"}, {"type": "qcode", "data": "comma"}]})
        elif ch == '&':
            send_qmp("send-key", {"keys": [{"type": "qcode", "data": "shift"}, {"type": "qcode", "data": "7"}]})
        elif ch == '_':
            send_qmp("send-key", {"keys": [{"type": "qcode", "data": "shift"}, {"type": "qcode", "data": "minus"}]})
        else:
            key = ch.lower()
            if ch.isupper():
                send_qmp("send-key", {"keys": [{"type": "qcode", "data": "shift"}, {"type": "qcode", "data": key}]})
            else:
                send_qmp("send-key", {"keys": [{"type": "qcode", "data": key}]})
        time.sleep(0.06)

print("Dismissing welcome dialog...")
send_qmp("send-key", {"keys": [{"type": "qcode", "data": "ret"}]})
time.sleep(1.5)

print("1. Testing Environment Variables & $VAR expansion...")
type_string("export GREET=MourosPower\n")
time.sleep(0.6)
type_string("echo Welcome to $GREET user=$USER\n")
time.sleep(0.8)
save_screen("/tmp/shell_1_env.ppm", f"{artifact_dir}/shell_1_env.png")

print("2. Testing Pipes (|) & Grep...")
type_string("ps | grep compositor\n")
time.sleep(0.8)
type_string("cat /etc/hostname | grep mouros\n")
time.sleep(0.8)
save_screen("/tmp/shell_2_pipes.ppm", f"{artifact_dir}/shell_2_pipes.png")

print("3. Testing Pipe + File Redirection (>) and Input Redirection (<)...")
type_string("ps | grep shell > /home/user/shell_task.txt\n")
time.sleep(0.8)
type_string("cat /home/user/shell_task.txt\n")
time.sleep(0.8)
type_string("grep shell < /home/user/shell_task.txt\n")
time.sleep(0.8)
save_screen("/tmp/shell_3_redir.ppm", f"{artifact_dir}/shell_3_redir.png")

print("4. Testing Command History & Tab completion...")
type_string("history\n")
time.sleep(0.8)
type_string("hel\t\n")
time.sleep(0.8)
save_screen("/tmp/shell_4_history_tab.ppm", f"{artifact_dir}/shell_4_history_tab.png")

s.close()
proc.terminate()
proc.wait(timeout=5)

print("\n--- Kernel Serial Log ---")
if os.path.exists(log_path):
    with open(log_path, "r", errors="ignore") as f:
        print(f.read())

print("SHELL UX VERIFICATION COMPLETE!")
