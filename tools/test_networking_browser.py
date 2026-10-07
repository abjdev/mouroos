import http.server
import json
import os
import socket
import socketserver
import subprocess
import sys
import threading
import time

artifact_dir = "/home/atahan/.gemini/antigravity/brain/d0ef7bc6-65c7-462b-9c92-f6e3a4c8768b"
sock_path = "/tmp/qmp-net.sock"
log_path = "/tmp/qemu_net.log"

if os.path.exists(sock_path):
    os.remove(sock_path)
if os.path.exists(log_path):
    os.remove(log_path)

# 1. Start Python HTTP Server
HTML_PAGE1 = """<!DOCTYPE html>
<html>
<head><title>Mouros OS Network Test</title></head>
<body>
<h1>Welcome to Mouros Browser!</h1>
<p>This web page was delivered over a real <strong>RTL8139 PCI</strong> interface and TCP/IPv4 stack.</p>
<hr>
<h2>Available Links</h2>
<ul>
  <li><a href="http://10.0.2.2:8000/page2.html">Click here to navigate to Page 2</a></li>
  <li><a href="http://10.0.2.2:8000/info.html">Hardware & Protocol Details</a></li>
</ul>
<p>Features active: Ethernet II &bull; ARP &bull; IPv4 &bull; TCP Stream &bull; HTML Parser &bull; GUI Renderer</p>
</body>
</html>
"""

HTML_PAGE2 = """<!DOCTYPE html>
<html>
<head><title>Page 2 - Navigation Succeeded</title></head>
<body>
<h1>Page 2 Loaded Successfully!</h1>
<p>Hyperlink navigation over TCP socket connection verified.</p>
<hr>
<p><a href="http://10.0.2.2:8000/">Back to Home Page</a></p>
</body>
</html>
"""

class CustomHTTPHandler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        print(f"[HTTP SERVER] Received GET request for path: {self.path}")
        if self.path == "/" or self.path == "/index.html":
            body = HTML_PAGE1.encode("utf-8")
        elif self.path == "/page2.html":
            body = HTML_PAGE2.encode("utf-8")
        else:
            body = f"<html><body><h1>Resource: {self.path}</h1></body></html>".encode("utf-8")

        self.send_response(200)
        self.send_header("Content-Type", "text/html; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Connection", "close")
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, format, *args):
        # Suppress verbose default logging
        pass

class ReusableTCPServer(socketserver.TCPServer):
    allow_reuse_address = True

httpd = ReusableTCPServer(("0.0.0.0", 8000), CustomHTTPHandler)
http_thread = threading.Thread(target=httpd.serve_forever, daemon=True)
http_thread.start()
print("[HTTP SERVER] Listening on http://0.0.0.0:8000/")

# 2. Launch QEMU with RTL8139 NIC
if not os.path.exists("disk.img"):
    subprocess.run(["qemu-img", "create", "-f", "raw", "disk.img", "32M"], check=True)

qemu_cmd = [
    "qemu-system-x86_64",
    "-cdrom", "mouros.iso",
    "-boot", "d",
    "-drive", "format=raw,file=disk.img,index=0,media=disk",
    "-m", "1G",
    "-vga", "std",
    "-netdev", "user,id=net0",
    "-device", "rtl8139,netdev=net0",
    "-serial", f"file:{log_path}",
    "-qmp", f"unix:{sock_path},server,nowait",
    "-display", "none",
    "-no-reboot"
]

print("[QEMU] Spawning QEMU with RTL8139 NIC...")
proc = subprocess.Popen(qemu_cmd)
time.sleep(3.5)

# Connect to QMP
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.connect(sock_path)
s.recv(4096) # greeting
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

def save_screen(name):
    ppm = f"/tmp/{name}.ppm"
    png = os.path.join(artifact_dir, f"{name}.png")
    send_qmp("screendump", {"filename": ppm})
    time.sleep(0.2)
    subprocess.run(["convert", ppm, png], stderr=subprocess.DEVNULL)
    print(f"[SCREENSHOT] Saved: {png}")

def send_key(key):
    send_qmp("send-key", {"keys": [{"type": "qcode", "data": key}]})

def send_shift_key(key):
    send_qmp("send-key", {"keys": [{"type": "qcode", "data": "shift"}, {"type": "qcode", "data": key}]})

def type_string(text):
    for ch in text:
        if ch == '\n':
            send_key("ret")
        elif ch == '\t':
            send_key("tab")
        elif ch == ' ':
            send_key("spc")
        elif ch == '.':
            send_key("dot")
        elif ch == '-':
            send_key("minus")
        elif ch == '/':
            send_key("slash")
        elif ch == '=':
            send_key("equal")
        elif ch == ':':
            send_shift_key("semicolon")
        elif ch == '_':
            send_shift_key("minus")
        elif ch == '|':
            send_shift_key("backslash")
        elif ch == '<':
            send_shift_key("comma")
        elif ch == '>':
            send_shift_key("dot")
        elif ch == '&':
            send_shift_key("7")
        elif ch == '$':
            send_shift_key("4")
        elif ch.isdigit():
            send_key(ch)
        elif ch.isupper():
            send_shift_key(ch.lower())
        else:
            send_key(ch)
        time.sleep(0.05)

# Wait for desktop ready
time.sleep(1.0)

# Check serial log
with open(log_path, "r", errors="ignore") as f:
    log_content = f.read()
    print("=== SERIAL OUTPUT PREVIEW ===")
    for line in log_content.splitlines():
        if "RTL8139" in line or "NET" in line or "DHCP" in line:
            print(f"  {line}")
    print("=============================")

# 1. Test ifconfig
print("[TEST] Running 'ifconfig'...")
type_string("ifconfig\n")
time.sleep(1.0)
save_screen("net_1_ifconfig")

# 2. Test ping
print("[TEST] Running 'ping -c 3 10.0.2.2'...")
type_string("ping -c 3 10.0.2.2\n")
time.sleep(1.5)
save_screen("net_2_ping")

# 3. Test curl
print("[TEST] Running 'curl -v http://10.0.2.2:8000/'...")
type_string("curl -v http://10.0.2.2:8000/\n")
time.sleep(1.5)
save_screen("net_3_curl")

# 4. Launch Mouros Browser
print("[TEST] Running 'browser http://10.0.2.2:8000/'...")
type_string("browser http://10.0.2.2:8000/\n")
time.sleep(2.5)
save_screen("net_4_browser_rendered")

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

# 5. Click the link in Mouros Browser to navigate to page2.html
print("[TEST] Calibrating mouse and clicking link to Page 2 at (265, 275)...")
calibrate_mouse_to_zero()
mouse_click(265, 275)
time.sleep(2.0)
save_screen("net_5_browser_page2")

# 6. Click Start Menu button at (30, 586) to show Browser in start menu and network indicator in tray
print("[TEST] Clicking Start Menu button at (30, 586)...")
mouse_click(30, 586)
time.sleep(0.8)
save_screen("net_6_desktop_and_tray")

# 7. Test HTTPS Warning Banner in Browser
print("[TEST] Testing HTTPS navigation in Browser Address Bar...")
# First click dismisses Start Menu
mouse_click(300, 83)
time.sleep(0.3)
# Second click focuses address bar
mouse_click(300, 83)
time.sleep(0.3)
# Clear address bar by sending backspaces
for _ in range(45):
    send_key("backspace")
    time.sleep(0.01)
# Type https URL
type_string("https://example.com/\n")
time.sleep(1.0)
save_screen("net_7_browser_https_warning")

print("[TEST] All tests completed! Terminating QEMU...")
proc.terminate()
try:
    proc.wait(timeout=3)
except:
    proc.kill()

httpd.shutdown()
print("[TEST] Verification finished successfully.")
