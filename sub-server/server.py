#!/usr/bin/env python3
"""
Subscription Link Proxy Server
Creates time-limited shareable links that proxy subscription content.
"""

import json, time, os, hashlib, secrets, urllib.request, ssl
from http.server import HTTPServer, BaseHTTPRequestHandler
from urllib.parse import urlparse, parse_qs
from datetime import datetime, timedelta

DATA_FILE = os.path.expanduser("~/.sub-server/data.json")
HOST = "0.0.0.0"
PORT = 18889
ADMIN_PASSWORD = "wwssaadd"

ssl_ctx = ssl.create_default_context()
ssl_ctx.check_hostname = False
ssl_ctx.verify_mode = ssl.CERT_NONE

def load_data():
    os.makedirs(os.path.dirname(DATA_FILE), exist_ok=True)
    if os.path.exists(DATA_FILE):
        return json.load(open(DATA_FILE))
    return {"links": {}}

def save_data(data):
    os.makedirs(os.path.dirname(DATA_FILE), exist_ok=True)
    json.dump(data, open(DATA_FILE, "w"), indent=2)

DURATIONS = {
    "1m": 60, "5m": 300, "10m": 600, "1h": 3600,
    "1d": 86400, "3d": 259200, "7d": 604800,
    "30d": 2592000, "365d": 31536000, "forever": -1,
}

class Handler(BaseHTTPRequestHandler):
    def log_message(self, *args): pass

    def json(self, data, code=200):
        self.send_response(code)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Access-Control-Allow-Origin", "*")
        self.end_headers()
        self.wfile.write(json.dumps(data, ensure_ascii=False).encode())

    def check_auth(self):
        q = parse_qs(urlparse(self.path).query)
        pw = q.get("pw", [""])[0]
        return pw == ADMIN_PASSWORD

    def do_GET(self):
        path = urlparse(self.path).path

        # Serve subscription content
        if path.startswith("/s/"):
            token = path[3:]
            data = load_data()
            link = data["links"].get(token)
            if not link:
                return self.json({"error": "Link not found"}, 404)

            if link["expires_at"] and time.time() > link["expires_at"]:
                del data["links"][token]
                save_data(data)
                return self.json({"error": "Link expired"}, 410)

            # Fetch and return subscription content
            try:
                req = urllib.request.Request(link["source_url"], headers={"User-Agent": "SubProxy/1.0"})
                resp = urllib.request.urlopen(req, timeout=15, context=ssl_ctx)
                body = resp.read()
                self.send_response(200)
                self.send_header("Content-Type", "text/plain; charset=utf-8")
                self.send_header("Access-Control-Allow-Origin", "*")
                self.end_headers()
                self.wfile.write(body)
            except Exception as e:
                return self.json({"error": f"Fetch failed: {e}"}, 502)
            return

        # API: list links
        if path == "/api/links":
            if not self.check_auth(): return self.json({"error": "Unauthorized"}, 401)
            data = load_data()
            links = []
            for token, link in data["links"].items():
                remaining = None
                if link["expires_at"]:
                    r = link["expires_at"] - time.time()
                    remaining = max(0, int(r)) if r > 0 else 0
                links.append({
                    "token": token,
                    "name": link["name"],
                    "source_url": link["source_url"][:50] + "...",
                    "duration": link["duration"],
                    "created_at": link["created_at"],
                    "expires_at": link["expires_at"],
                    "remaining_seconds": remaining,
                    "service_url": f"http://{self.headers.get('Host', 'localhost')}/s/{token}",
                })
            return self.json({"links": links})

        # Web UI
        if path in ("/", "/admin"):
            return self.serve_ui()

        return self.json({"error": "Not found"}, 404)

    def do_POST(self):
        path = urlparse(self.path).path

        # Create link
        if path == "/api/create":
            if not self.check_auth(): return self.json({"error": "Unauthorized"}, 401)
            length = int(self.headers.get("Content-Length", 0))
            body = json.loads(self.rfile.read(length))
            source_url = body.get("source_url", "").strip()
            duration = body.get("duration", "1d")
            name = body.get("name", "").strip()

            if not source_url:
                return self.json({"error": "source_url required"}, 400)
            if duration not in DURATIONS:
                return self.json({"error": f"Invalid duration: {duration}"}, 400)

            token = secrets.token_urlsafe(8)
            seconds = DURATIONS[duration]
            expires_at = None if seconds == -1 else time.time() + seconds

            data = load_data()
            data["links"][token] = {
                "name": name or f"Link-{token[:6]}",
                "source_url": source_url,
                "duration": duration,
                "token": token,
                "created_at": time.time(),
                "expires_at": expires_at,
            }
            save_data(data)

            return self.json({"ok": True, "token": token, "expires_at": expires_at})

        # Delete link
        if path == "/api/delete":
            if not self.check_auth(): return self.json({"error": "Unauthorized"}, 401)
            length = int(self.headers.get("Content-Length", 0))
            body = json.loads(self.rfile.read(length))
            token = body.get("token", "")

            data = load_data()
            if token in data["links"]:
                del data["links"][token]
                save_data(data)
                return self.json({"ok": True})
            return self.json({"error": "Not found"}, 404)

        return self.json({"error": "Not found"}, 404)

    def do_OPTIONS(self):
        self.send_response(200)
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "GET, POST, DELETE, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "Content-Type")
        self.end_headers()

    def serve_ui(self):
        html = """<!DOCTYPE html><html lang="zh"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Sub Proxy</title>
<style>*,*::before,*::after{box-sizing:border-box;margin:0;padding:0}
body{font-family:system-ui,-apple-system,sans-serif;background:#08080c;color:#e4e4e7;min-height:100vh}
.header{background:#0c0c12;border-bottom:1px solid rgba(255,255,255,.06);padding:12px 20px;display:flex;align-items:center;justify-content:space-between}
.header h1{font-size:16px;font-weight:700}.header h1 span{color:#6366f1}
.container{max-width:960px;margin:0 auto;padding:24px 20px}
.card{background:#0c0c12;border:1px solid rgba(255,255,255,.06);border-radius:12px;padding:20px;margin-bottom:16px}
.card h3{font-size:14px;font-weight:600;margin-bottom:16px;display:flex;align-items:center;gap:8px}
.btn{padding:8px 16px;border-radius:8px;border:none;font-size:13px;font-weight:600;cursor:pointer;transition:all .15s}
.btn-primary{background:#6366f1;color:#fff}.btn-primary:hover{background:#818cf8}
.btn-danger{background:rgba(239,68,68,.15);color:#ef4444;border:1px solid rgba(239,68,68,.2)}.btn-danger:hover{background:rgba(239,68,68,.25)}
.btn-sm{padding:4px 10px;font-size:11px}
.input{width:100%;padding:8px 12px;background:#0a0a0f;border:1px solid rgba(255,255,255,.06);border-radius:8px;color:#d4d4d8;font-size:13px;outline:none}.input:focus{border-color:rgba(99,102,241,.4)}
select.input{cursor:pointer}
.row{display:flex;gap:8px;margin-bottom:12px}.row>*{flex:1}
.badge{display:inline-flex;padding:2px 8px;border-radius:4px;font-size:10px;font-weight:600;text-transform:uppercase}
.badge-green{background:rgba(52,211,153,.1);color:#34d399}.badge-red{background:rgba(239,68,68,.1);color:#ef4444}.badge-muted{background:rgba(113,113,122,.1);color:#a1a1aa}
table{width:100%;border-collapse:collapse;font-size:12px}
th,td{text-align:left;padding:10px 8px;border-bottom:1px solid rgba(255,255,255,.04)}
th{color:#a1a1aa;font-weight:500;font-size:11px;text-transform:uppercase}td{color:#d4d4d8}
.mono{font-family:'SF Mono',Fira Code,monospace;font-size:11px}
.copy-btn{cursor:pointer;color:#6366f1;font-size:11px}.copy-btn:hover{color:#818cf8}
.status{display:inline-block;width:8px;height:8px;border-radius:50%;margin-right:6px}
.status-active{background:#34d399}.status-expired{background:#ef4444}
.toast{position:fixed;top:16px;right:16px;padding:10px 18px;border-radius:8px;font-size:13px;z-index:999;animation:slideIn .2s}
.toast-success{background:#065f46;color:#6ee7b7}.toast-error{background:#7f1d1d;color:#fca5a5}
@keyframes slideIn{from{transform:translateX(100%);opacity:0}to{transform:translateX(0);opacity:1}}
</style></head><body>
<div class="header"><h1>Sub<span>Proxy</span></h1><div></div></div>
<div class="container">
<div class="card"><h3>Create Time-Limited Link</h3>
<div class="row"><input class="input" id="sourceUrl" placeholder="Subscription URL (e.g. https://pro.dl.214578.xyz/sub?token=...)"></div>
<div class="row"><input class="input" id="linkName" placeholder="Link name (optional)"></div>
<div class="row">
<select class="input" id="duration">
<option value="1m">1 Minute</option><option value="5m">5 Minutes</option><option value="10m">10 Minutes</option>
<option value="1h">1 Hour</option><option value="1d" selected>1 Day</option><option value="3d">3 Days</option>
<option value="7d">1 Week</option><option value="30d">1 Month</option><option value="365d">1 Year</option>
<option value="forever">Forever</option>
</select>
<input class="input" id="adminPw" type="password" placeholder="Admin password" style="max-width:160px">
<button class="btn btn-primary" onclick="createLink()" style="flex:none">Create</button>
</div></div>
<div class="card"><h3>Active Links</h3><table><thead><tr><th>Name</th><th>Duration</th><th>Expires</th><th>URL</th><th></th></tr></thead><tbody id="linkList"><tr><td colspan="5" style="text-align:center;color:#71717a">Loading...</td></tr></tbody></table></div>
</div>
<script>
const pw = () => document.getElementById('adminPw').value;
const API = (url, opts={}) => fetch(url+'?pw='+encodeURIComponent(pw()), opts).then(r=>r.json());
async function createLink(){
  const source_url = document.getElementById('sourceUrl').value.trim();
  const name = document.getElementById('linkName').value.trim();
  const duration = document.getElementById('duration').value;
  if(!source_url) return toast('Please enter a subscription URL', 'error');
  try{
    const r = await API('/api/create', {method:'POST', body:JSON.stringify({source_url, name, duration}), headers:{'Content-Type':'application/json'}});
    if(r.ok){document.getElementById('sourceUrl').value='';document.getElementById('linkName').value='';refreshLinks();toast('Link created!','success');}
    else toast(r.error,'error');
  }catch(e){toast('Error: '+e,'error');}
}
async function deleteLink(token){
  try{
    const r = await API('/api/delete', {method:'POST', body:JSON.stringify({token}), headers:{'Content-Type':'application/json'}});
    if(r.ok) refreshLinks();
  }catch(e){}
}
async function refreshLinks(){
  try{
    const r = await API('/api/links');
    if(r.error) return;
    const tbody = document.getElementById('linkList');
    if(!r.links.length){tbody.innerHTML='<tr><td colspan="5" style="text-align:center;color:#71717a">No links yet</td></tr>';return;}
    tbody.innerHTML = r.links.map(l => {
      const status = l.remaining_seconds === null ? '<span class="status status-active"></span>Permanent' : l.remaining_seconds > 0 ? '<span class="status status-active"></span>Active' : '<span class="status status-expired"></span>Expired';
      const remaining = l.remaining_seconds === null ? '∞' : l.remaining_seconds > 86400 ? Math.ceil(l.remaining_seconds/86400)+'d' : l.remaining_seconds > 3600 ? Math.ceil(l.remaining_seconds/3600)+'h' : Math.ceil(l.remaining_seconds/60)+'m';
      return `<tr><td>${esc(l.name)}</td><td>${l.duration}</td><td>${status} ${remaining}</td><td><span class="mono">${esc(l.service_url)}</span> <span class="copy-btn" onclick="copy('${l.service_url}')">Copy</span></td><td><button class="btn btn-danger btn-sm" onclick="deleteLink('${l.token}')">Delete</button></td></tr>`;
    }).join('');
  }catch(e){}
}
function esc(s){return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');}
function copy(text){navigator.clipboard.writeText(text);toast('Copied!','success');}
function toast(msg,type){const t=document.createElement('div');t.className='toast toast-'+type;t.textContent=msg;document.body.appendChild(t);setTimeout(()=>t.remove(),2500);}
refreshLinks();setInterval(refreshLinks, 10000);
</script></body></html>"""
        self.send_response(200)
        self.send_header("Content-Type", "text/html; charset=utf-8")
        self.end_headers()
        self.wfile.write(html.encode())

if __name__ == "__main__":
    print(f"Sub Proxy Server starting on {HOST}:{PORT}")
    server = HTTPServer((HOST, PORT), Handler)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        server.shutdown()
