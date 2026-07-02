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
        if path == "/test":
            return self.serve_test()

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
        html = """<!DOCTYPE html><html lang="zh-CN"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>订阅代理 · 限时链接管理</title>
<style>*,*::before,*::after{box-sizing:border-box;margin:0;padding:0}
@keyframes fadeIn{from{opacity:0;transform:translateY(8px)}to{opacity:1;transform:translateY(0)}}
@keyframes slideIn{from{transform:translateX(100%);opacity:0}to{transform:translateX(0);opacity:1}}
@keyframes pulse{0%,100%{opacity:1}50%{opacity:.6}}
@keyframes shimmer{0%{background-position:-200% 0}100%{background-position:200% 0}}
@keyframes scaleIn{from{transform:scale(.9);opacity:0}to{transform:scale(1);opacity:1}}
body{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,"PingFang SC","Microsoft YaHei",sans-serif;background:#060608;color:#e4e4e7;min-height:100vh}
.grain{position:fixed;inset:0;opacity:.02;background:url("data:image/svg+xml,%3Csvg viewBox='0 0 256 256' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence baseFrequency='.8'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)' opacity='.5'/%3E%3C/svg%3E");pointer-events:none;z-index:0}
.header{position:sticky;top:0;z-index:10;background:rgba(10,10,14,.85);backdrop-filter:blur(20px);-webkit-backdrop-filter:blur(20px);border-bottom:1px solid rgba(255,255,255,.06);padding:14px 24px;display:flex;align-items:center;justify-content:space-between}
.header-left{display:flex;align-items:center;gap:10px}
.logo{width:28px;height:28px;border-radius:8px;background:linear-gradient(135deg,#6366f1,#8b5cf6);display:flex;align-items:center;justify-content:center;font-size:15px;box-shadow:0 4px 12px rgba(99,102,241,.25)}
.header h1{font-size:16px;font-weight:700;letter-spacing:-.3px}.header h1 span{background:linear-gradient(135deg,#a5b4fc,#c4b5fd);-webkit-background-clip:text;-webkit-text-fill-color:transparent}
.header-status{display:flex;align-items:center;gap:6px;font-size:12px;color:#a1a1aa}.header-status .dot{width:6px;height:6px;border-radius:50%;background:#34d399;animation:pulse 2s infinite}
.container{max-width:1040px;margin:0 auto;padding:28px 24px;position:relative;z-index:1}
.card{background:rgba(15,15,20,.7);border:1px solid rgba(255,255,255,.05);border-radius:16px;padding:24px;margin-bottom:20px;backdrop-filter:blur(12px);-webkit-backdrop-filter:blur(12px);animation:fadeIn .4s ease both}
.card:nth-child(2){animation-delay:.05s}.card:nth-child(3){animation-delay:.1s}
.card-header{display:flex;align-items:center;gap:10px;margin-bottom:20px}
.card-icon{width:36px;height:36px;border-radius:10px;display:flex;align-items:center;justify-content:center;font-size:17px}
.card-icon-create{background:rgba(99,102,241,.12);color:#818cf8}
.card-icon-list{background:rgba(52,211,153,.1);color:#34d399}
.card h3{font-size:15px;font-weight:600;letter-spacing:-.2px}
.card-subtitle{font-size:12px;color:#71717a;margin-top:1px}
.form-grid{display:grid;grid-template-columns:1fr 1fr;gap:12px;margin-bottom:12px}
.form-grid .full{grid-column:1/-1}
.input{width:100%;padding:10px 14px;background:rgba(0,0,0,.25);border:1px solid rgba(255,255,255,.06);border-radius:10px;color:#d4d4d8;font-size:13px;outline:none;transition:all .2s}.input:focus{border-color:rgba(99,102,241,.4);box-shadow:0 0 0 3px rgba(99,102,241,.08)}
select.input{cursor:pointer;appearance:none;background-image:url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' fill='%2371717a'%3E%3Cpath d='M6 8L1 3h10z'/%3E%3C/svg%3E");background-repeat:no-repeat;background-position:right 12px center;padding-right:32px}
select.input option{background:#1a1a20;color:#d4d4d8}
.btn{padding:10px 20px;border-radius:10px;border:none;font-size:13px;font-weight:600;cursor:pointer;transition:all .2s;display:inline-flex;align-items:center;gap:6px;letter-spacing:-.1px}
.btn:active{transform:scale(.97)}
.btn-primary{background:linear-gradient(135deg,#6366f1,#5b5eeb);color:#fff;box-shadow:0 2px 8px rgba(99,102,241,.25)}.btn-primary:hover{box-shadow:0 4px 16px rgba(99,102,241,.35);transform:translateY(-1px)}
.btn-ghost{background:rgba(255,255,255,.04);color:#a1a1aa;border:1px solid rgba(255,255,255,.06)}.btn-ghost:hover{background:rgba(255,255,255,.08);color:#d4d4d8}
.btn-danger{background:rgba(239,68,68,.1);color:#f87171;border:1px solid rgba(239,68,68,.15);padding:6px 12px;font-size:12px}.btn-danger:hover{background:rgba(239,68,68,.2);color:#fca5a5}
.btn-sm{padding:5px 10px;font-size:11px;border-radius:8px}
table{width:100%;border-collapse:collapse}
thead th{text-align:left;padding:10px 12px;color:#71717a;font-size:11px;font-weight:600;text-transform:uppercase;letter-spacing:.5px;border-bottom:1px solid rgba(255,255,255,.04)}
tbody td{padding:12px;font-size:13px;border-bottom:1px solid rgba(255,255,255,.03);vertical-align:middle}
tbody tr{transition:all .15s;animation:fadeIn .3s ease both}
tbody tr:hover{background:rgba(255,255,255,.02)}
tbody tr:last-child td{border-bottom:none}
.badge{display:inline-flex;align-items:center;gap:5px;padding:3px 10px;border-radius:20px;font-size:11px;font-weight:600}
.badge-active{background:rgba(52,211,153,.1);color:#34d399;border:1px solid rgba(52,211,153,.15)}
.badge-expired{background:rgba(239,68,68,.08);color:#f87171;border:1px solid rgba(239,68,68,.1)}
.badge-permanent{background:rgba(168,85,247,.1);color:#a78bfa;border:1px solid rgba(168,85,247,.15)}
.badge-dot{width:5px;height:5px;border-radius:50%}.badge-active .badge-dot{background:#34d399}.badge-expired .badge-dot{background:#f87171}.badge-permanent .badge-dot{background:#a78bfa}
.mono{font-family:"SF Mono","Fira Code","Cascadia Code",monospace;font-size:12px;color:#a1a1aa;word-break:break-all}
.copy-link{color:#818cf8;cursor:pointer;font-size:12px;margin-left:8px;white-space:nowrap;transition:color .15s}.copy-link:hover{color:#a5b4fc}
.empty-state{text-align:center;padding:40px 20px;color:#52525b}
.empty-state-icon{font-size:40px;margin-bottom:12px;opacity:.5}
.toast{position:fixed;top:20px;right:20px;padding:12px 20px;border-radius:12px;font-size:13px;z-index:999;animation:slideIn .3s ease;backdrop-filter:blur(12px);-webkit-backdrop-filter:blur(12px)}
.toast-success{background:rgba(5,150,105,.9);color:#d1fae5;border:1px solid rgba(52,211,153,.3)}.toast-error{background:rgba(185,28,28,.9);color:#fecaca;border:1px solid rgba(239,68,68,.3)}
.time-display{font-variant-numeric:tabular-nums}
@media(max-width:640px){.form-grid{grid-template-columns:1fr}.header{padding:12px 16px}.container{padding:16px}.card{padding:16px}}
</style></head><body>
<div class="grain"></div>
<div class="header"><div class="header-left"><div class="logo">⚡</div><h1>订阅<span>代理</span></h1></div><div class="header-status"><span class="dot"></span>服务运行中</div></div>
<div id="jsErr" style="max-width:1040px;margin:0 auto;padding:0 24px"></div>
<div class="container">
<div class="card"><div class="card-header"><div class="card-icon card-icon-create">🔗</div><div><h3>创建限时链接</h3><p class="card-subtitle">填入订阅地址，生成带过期时间的分享链接</p></div></div>
<div class="form-grid"><input class="input full" id="sourceUrl" placeholder="订阅地址，例如 https://pro.dl.214578.xyz/sub?token=..."><input class="input" id="linkName" placeholder="链接名称（选填）"></div>
<div class="form-grid"><select class="input" id="duration"><option value="1m">⏱ 1 分钟</option><option value="5m">⏱ 5 分钟</option><option value="10m">⏱ 10 分钟</option><option value="1h">🕐 1 小时</option><option value="1d" selected>📅 1 天</option><option value="3d">📅 3 天</option><option value="7d">📅 1 周</option><option value="30d">📅 1 个月</option><option value="365d">📅 1 年</option><option value="forever">♾️ 永久</option></select><input class="input" id="adminPw" type="password" placeholder="管理密码" style="max-width:160px"><button class="btn btn-primary" id="createBtn" onclick="createLink()">✦ 创建链接</button></div></div>
<div class="card"><div class="card-header"><div class="card-icon card-icon-list">📋</div><div><h3>已创建的链接</h3><p class="card-subtitle" id="linkCount">加载中...</p></div></div><table><thead><tr><th>名称</th><th>时长</th><th>状态</th><th>链接地址</th><th></th></tr></thead><tbody id="linkList"><tr><td colspan="5"><div class="empty-state"><div class="empty-state-icon">⏳</div><p>加载中...</p></div></td></tr></tbody></table></div>
</div>
<script>
window.onerror = function(msg, url, line){document.getElementById('jsErr').innerHTML += '<div style=color:red>JS ERROR: '+msg+' line '+line+'</div>';return false};
const pw = () => document.getElementById('adminPw').value || localStorage.getItem('subproxy_pw') || '';
document.getElementById('adminPw').addEventListener('input', function(){localStorage.setItem('subproxy_pw', this.value)});
document.getElementById('adminPw').value = localStorage.getItem('subproxy_pw') || '';
const API = (url, opts={}) => fetch(url+'?pw='+encodeURIComponent(pw()), opts).then(r=>{if(!r.ok)throw new Error('HTTP '+r.status);return r.json()});
async function createLink(){
  const source_url = document.getElementById('sourceUrl').value.trim();
  const name = document.getElementById('linkName').value.trim();
  const duration = document.getElementById('duration').value;
  if(!source_url) return toast('请填写订阅地址','error');
  if(!pw()) return toast('请输入管理密码','error');
  const btn = document.getElementById('createBtn'); btn.disabled = true; btn.textContent = '创建中...';
  try{
    const r = await API('/api/create', {method:'POST', body:JSON.stringify({source_url, name, duration}), headers:{'Content-Type':'application/json'}});
    if(r.ok){document.getElementById('sourceUrl').value='';document.getElementById('linkName').value='';refreshLinks();toast('链接创建成功！','success')}
    else toast(r.error,'error');
  }catch(e){toast('网络错误: '+e,'error')}
  finally{btn.disabled = false; btn.textContent = '✦ 创建链接'}
}
async function deleteLink(token){
  if(!confirm('确定要删除这个链接吗？')) return;
  try{const r = await API('/api/delete', {method:'POST', body:JSON.stringify({token}), headers:{'Content-Type':'application/json'}});if(r.ok) refreshLinks();else toast(r.error,'error')}catch(e){}
}
function formatTime(seconds){
  if(seconds === null) return '∞';
  if(seconds <= 0) return '已过期';
  const d = Math.floor(seconds/86400), h = Math.floor((seconds%86400)/3600), m = Math.floor((seconds%3600)/60);
  if(d>0) return d+'天'+h+'小时';
  if(h>0) return h+'小时'+m+'分钟';
  if(m>0) return m+'分钟';
  return Math.floor(seconds)+'秒';
}
function durationLabel(d){
  const map = {'1m':'1分钟','5m':'5分钟','10m':'10分钟','1h':'1小时','1d':'1天','3d':'3天','7d':'1周','30d':'1个月','365d':'1年','forever':'永久'};
  return map[d] || d;
}
async function refreshLinks(){
  try{
    const r = await API('/api/links');
    if(r.error){document.getElementById('linkList').innerHTML='<tr><td colspan="5"><div class="empty-state"><div class="empty-state-icon">🔒</div><p>请输入密码查看</p></div></td></tr>';return}
    const tbody = document.getElementById('linkList');
    document.getElementById('linkCount').textContent = '共 ' + (r.links?r.links.length:0) + ' 个链接';
    if(!r.links||!r.links.length){tbody.innerHTML='<tr><td colspan="5"><div class="empty-state"><div class="empty-state-icon">📭</div><p>暂无链接</p><p style="font-size:12px;margin-top:4px">在上方创建你的第一个限时链接</p></div></td></tr>';return}
    tbody.innerHTML = r.links.map((l,i) => {
      const expired = l.remaining_seconds !== null && l.remaining_seconds <= 0;
      const permanent = l.remaining_seconds === null;
      const badgeCls = expired ? 'badge-expired' : permanent ? 'badge-permanent' : 'badge-active';
      const statusText = expired ? '已过期' : permanent ? '永久有效' : '有效';
      const remainingText = expired ? '' : ' · 剩余 '+formatTime(l.remaining_seconds);
      return '<tr style="animation-delay:'+(i*.03)+'s"><td>'+esc(l.name)+'</td><td>'+durationLabel(l.duration)+'</td><td><span class="badge '+badgeCls+'"><span class="badge-dot"></span>'+statusText+remainingText+'</span></td><td><span class="mono">'+esc(l.service_url)+'</span><span class="copy-link" onclick="copy(\''+l.service_url+'\')">⎘ 复制</span></td><td><button class="btn btn-danger btn-sm" onclick="deleteLink(\''+l.token+'\')">删除</button></td></tr>'
    }).join('');
  }catch(e){}
}
function esc(s){return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;')}
function copy(text){navigator.clipboard.writeText(text).then(()=>toast('已复制到剪贴板','success')).catch(()=>toast('复制失败','error'))}
function toast(msg,type){const t=document.createElement('div');t.className='toast toast-'+type;t.textContent=msg;document.body.appendChild(t);setTimeout(()=>{t.style.opacity='0';t.style.transition='opacity .3s';setTimeout(()=>t.remove(),300)},2200)}
refreshLinks();setInterval(refreshLinks, 15000);
</script></body></html>"""
        self.send_response(200)
        self.send_header("Content-Type", "text/html; charset=utf-8")
        self.end_headers()
        self.wfile.write(html.encode())

    def serve_test(self):
        html = """<!DOCTYPE html><html><body>
<h1>Test Page</h1>
<button onclick="alert('clicked works')">Test Alert</button>
<button onclick="fetch('/api/create?pw=wwssaadd',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({source_url:'https://example.com',duration:'1d',name:'test'})}).then(r=>r.json()).then(d=>alert('OK: '+JSON.stringify(d))).catch(e=>alert('ERR: '+e))">Test API</button>
</body></html>"""
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
