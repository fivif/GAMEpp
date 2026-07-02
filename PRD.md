# GAME++ 游戏加速器 — 产品需求文档 (PRD)

> **版本**: v1.0  
> **日期**: 2026-07-03  
> **技术栈**: Rust + Tauri  
> **目标平台**: Windows 10/11 (主要) + macOS 12+  
> **基础设施**: VLESS + WebSocket + TLS 代理节点订阅

---

## 目录

1. [产品概述](#1-产品概述)
2. [节点资源现状](#2-节点资源现状)
3. [核心功能](#3-核心功能)
4. [系统架构](#4-系统架构)
5. [技术方案](#5-技术方案)
6. [详细功能说明](#6-详细功能说明)
7. [开发路线图](#7-开发路线图)
8. [Rust Crate 依赖](#8-rust-crate-依赖)
9. [UI 设计概要](#9-ui-设计概要)
10. [风险评估](#10-风险评估)

---

## 1. 产品概述

### 1.1 产品定位

**GAME++** 是一款面向游戏玩家的桌面网络加速工具。利用 VLESS 代理节点，将游戏/应用的网络流量通过优化的跨境路径转发，降低延迟和丢包，解决跨境游戏的网络卡顿问题。

### 1.2 与商业加速器的差异化

| 维度 | 商业加速器（UU/迅游） | GAME++ |
|------|----------------------|--------|
| 节点资源 | 自建专线 + BGP机房 | 用户自有的 VLESS 订阅节点 |
| 成本 | 月费 30-50 元 | 仅节点订阅费 |
| 开放性 | 闭源黑盒 | 完全开源 |
| 定制性 | 固定游戏列表 | 可自定义任意应用/游戏/IP段 |
| 平台 | Win为主 | Win + Mac |

### 1.3 目标用户

- 玩海外游戏的国内玩家
- 已有 VLESS/V2Ray 代理订阅的用户
- 希望按应用粒度（而非全局）使用代理的用户

---

## 2. 节点资源现状

### 2.1 订阅格式

```
VLESS UUID@IP:PORT?security=tls&type=ws&host=xxx&path=/&encryption=none#REGION|标签|延迟ms
```

### 2.2 节点分布

| 区域 | 地区 | 数量（约） | 典型延迟 | 主要端口 |
|------|------|-----------|---------|---------|
| **HK** | 香港 | 150+ | 60-238ms | 443, 8443, 2053, 2083, 2087, 2096 |
| **SG** | 新加坡 | 30+ | 79-220ms | 443, 8443, 2087 |
| **JP** | 日本 | 20+ | 83-139ms | 443, 8443, 2053, 2087 |
| **US** | 美国 | 40+ | 181-282ms | 443 |
| **DE** | 德国 | 20+ | 213-276ms | 443 |
| **NL** | 荷兰 | 12+ | 236-328ms | 443 |
| **CF** | Cloudflare CDN | 16 | 中国优化 | 2053, 2083, 2087, 2096 |

### 2.3 节点共性参数

```
协议:     VLESS (V2Ray)
传输:     WebSocket
TLS:      开启
Host/SNI: pro.dl.214578.xyz
路径:     /
指纹:     Chrome
UUID:     统一
```

### 2.4 节点选择策略（推荐）

对于中国大陆用户加速海外游戏：
- **亚洲游戏（日服/韩服/东南亚服）**: HK > JP > SG 节点
- **美服游戏**: US 直连节点
- **欧服游戏**: DE > NL 节点
- **通用加速**: CF 中国优化节点（Cloudflare CDN 回源）

---

## 3. 核心功能

### 3.1 功能矩阵

| 功能 | 优先级 | 描述 |
|------|--------|------|
| 订阅管理 | P0 | 支持 VLESS 订阅链接导入，自动解析节点列表 |
| 节点测速 | P0 | 并发 Ping / 真实 TCP 延迟测试，排序展示 |
| 本地 SOCKS5 代理 | P0 | 基于选定节点启动本地 SOCKS5 代理服务 |
| 系统代理开关 | P0 | 一键开启/关闭系统级 HTTP/HTTPS/SOCKS5 代理 |
| 应用白名单 | P1 | 自定义需要加速的 .exe/.app 列表，仅对指定应用走代理 |
| TUN 模式 | P1 | 创建虚拟网卡 + 路由表，代理全部流量（游戏不认系统代理时的方案） |
| IP 路由规则 | P1 | 自定义 IP/CIDR 段走代理，其余直连 |
| 游戏预设库 | P2 | 内置热门游戏（Valorant/LOL/Apex/CS2/原神等）的代理规则 |
| 延迟监控 | P2 | 实时显示当前延迟、丢包、流量图表 |
| 智能选节点 | P2 | 根据目标游戏服务器位置自动推荐最优节点 |
| 开机自启 | P2 | 系统托盘常驻 + 开机自动连接 |
| 配置导入导出 | P3 | 分享加速配置给其他人 |

### 3.2 加速模式对比

| 模式 | 原理 | 优点 | 缺点 | 适用场景 |
|------|------|------|------|---------|
| **系统代理** | 修改系统 HTTP/SOCKS5 代理设置 | 简单、稳定 | 大部分游戏不认系统代理 | 浏览器游戏、Epic/Steam 下载 |
| **TUN 模式** | 虚拟网卡 + 路由表劫持 | 100% 覆盖所有流量 | 需管理员权限 | 所有联网游戏 |
| **进程代理** | Hook/WFP 驱动按进程拦截 | 精确到进程 | 技术复杂，Win-only | 特定游戏加速 |

---

## 4. 系统架构

### 4.1 整体架构图

```
┌──────────────────────────────────────────────────────────┐
│                    GAME++ Desktop App                     │
│                                                          │
│  ┌──────────────┐  ┌──────────────────────────────────┐  │
│  │   Tauri UI   │  │         Rust Backend             │  │
│  │  (React/TS)  │  │                                  │  │
│  │              │  │  ┌────────────────────────────┐  │  │
│  │  - 节点列表   │  │  │    Subscription Manager    │  │  │
│  │  - 测速面板   │  │  │  - 订阅 URL 拉取           │  │  │
│  │  - 应用选择   │◄─┼──│  - Base64 解码             │  │  │
│  │  - 连接开关   │  │  │  - VLESS URL 解析          │  │  │
│  │  - 延迟图表   │  │  └────────────────────────────┘  │  │
│  └──────────────┘  │                                  │  │
│                    │  ┌────────────────────────────┐  │  │
│                    │  │    Proxy Engine (核心)      │  │  │
│                    │  │  - VLESS Client 实现        │  │  │
│                    │  │  - WebSocket 连接管理        │  │  │
│                    │  │  - TLS 握手 (rustls)       │  │  │
│                    │  │  - 本地 SOCKS5 Server       │  │  │
│                    │  │  - 连接池 + 多路复用         │  │  │
│                    │  └────────────────────────────┘  │  │
│                    │                                  │  │
│                    │  ┌────────────────────────────┐  │  │
│                    │  │    Traffic Router           │  │  │
│                    │  │  - TUN 虚拟网卡              │  │  │
│                    │  │  - 路由表管理               │  │  │
│                    │  │  - IP/CIDR 规则引擎         │  │  │
│                    │  │  - 进程匹配（WPF/NWE）       │  │  │
│                    │  └────────────────────────────┘  │  │
│                    │                                  │  │
│                    │  ┌────────────────────────────┐  │  │
│                    │  │    Latency Monitor          │  │  │
│                    │  │  - TCP Ping 并发测试         │  │  │
│                    │  │  - 实时流量统计             │  │  │
│                    │  │  - 节点健康度评分           │  │  │
│                    │  └────────────────────────────┘  │  │
│                    │                                  │  │
│                    │  ┌────────────────────────────┐  │  │
│                    │  │    Platform Adapter         │  │  │
│                    │  │  - Windows: netsh/注册表     │  │  │
│                    │  │  - macOS: networksetup      │  │  │
│                    │  │  - 系统托盘管理              │  │  │
│                    │  │  - 开机自启                  │  │  │
│                    │  └────────────────────────────┘  │  │
│                    └──────────────────────────────────┘  │
└──────────────────────────────────────────────────────────┘
         │                                          │
         ▼                                          ▼
┌──────────────────┐                    ┌─────────────────────┐
│  VLESS 代理节点   │ ◄──── TLS/WS ────► │   游戏服务器          │
│  (Cloudflare CDN) │                    │  (Valorant/LOL/...) │
└──────────────────┘                    └─────────────────────┘
```

### 4.2 数据流（以加速 Valorant 为例）

```
Valorant.exe
    │
    │ UDP packet (dst: 192.207.0.1:7777)
    ▼
┌─────────────────┐
│ 1. 流量捕获      │  TUN虚拟网卡 或 系统代理
│    packet读入    │
└────────┬────────┘
         ▼
┌─────────────────┐
│ 2. 规则匹配      │  匹配目标 IP → 走代理 or 直连
│   路由决策       │
└────────┬────────┘
         ▼  (走代理)
┌─────────────────┐
│ 3. SOCKS5 封装  │  SOCKS5 UDP ASSOCIATE → 本地代理
│    本地转发      │
└────────┬────────┘
         ▼
┌─────────────────┐
│ 4. VLESS 加密   │  VLESS 协议封装 + TLS 加密
│    WS 隧道      │  通过 WebSocket 发送
└────────┬────────┘
         ▼
┌─────────────────┐
│ 5. 代理节点     │  Cloudflare CDN → 后端服务器
│    转发解包      │
└────────┬────────┘
         ▼
┌─────────────────┐
│ 6. 游戏服务器   │  原始 UDP 包到达
│    返回数据     │
└─────────────────┘
         │
         ▼
    原路返回 → Valorant.exe 收到响应
```

---

## 5. 技术方案

### 5.1 VLESS 客户端实现方案

VLESS 协议相对简单（无加密层），核心流程：

```
1. 建立 WebSocket 连接（通过 TLS）
2. 发送 VLESS 握手包:
   [1 byte version][16 byte UUID][1 byte 附加协议长度][附加协议数据][目标地址]
   
   目标地址格式:
   [1 byte type(1=IPv4,2=域名,3=IPv6)][地址][2 byte port]
3. 握手成功后，后续数据裸传（VLESS 本身无加密）
4. 应用层数据直接通过 WebSocket 双向转发
```

**关键实现点**:
- WebSocket 连接复用：一个连接承载多路流量（通过连接池）
- 自动重连：断线后自动切换节点
- TLS 指纹伪装：使用 Chrome 指纹（`rustls` 可配置）

### 5.2 本地 SOCKS5 代理方案

```
本地应用 ──► SOCKS5 Server (127.0.0.1:1080) ──► VLESS Client ──► 远程节点
```

SOCKS5 支持：
- **TCP CONNECT**: 标准 TCP 代理（Steam下载、网页等）
- **UDP ASSOCIATE**: UDP 代理（游戏核心需求）

### 5.3 平台流量劫持方案

#### Windows 方案

| 阶段 | 方案 | 复杂度 |
|------|------|--------|
| **V1 - 系统代理** | 修改 `HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings` 注册表 + `netsh winhttp set proxy` | 低 |
| **V2 - TUN 模式** | 使用 `wintun.dll` 创建虚拟网卡，`route add` 添加路由规则 | 中 |
| **V3 - 进程级** | WFP (Windows Filtering Platform) 内核驱动按 PID 过滤 | 高 |

**推荐 V1+V2 组合**：系统代理用于日常 + TUN 模式用于游戏

#### macOS 方案

| 阶段 | 方案 | 复杂度 |
|------|------|--------|
| **V1 - 系统代理** | `networksetup -setwebproxy / -setsocksfirewallproxy` | 低 |
| **V2 - TUN 模式** | `utun` 设备 + `/sbin/route add` | 中 |
| **V3 - 进程级** | Network Extension Framework (`NETransparentProxy`) | 高 |

### 5.4 TUN 模式详细设计

```
┌─────────────────────────────────────────────┐
│              TUN Mode Data Flow              │
│                                              │
│  游戏进程                                     │
│     │                                        │
│     │ socket(AF_INET, SOCK_DGRAM, ...)       │
│     ▼                                        │
│  ┌──────────────────────┐                    │
│  │  操作系统网络栈       │                    │
│  │  (路由表已修改)       │                    │
│  └────────┬─────────────┘                    │
│           │ 路由匹配: 目标IP在代理列表中        │
│           ▼                                  │
│  ┌──────────────────────┐                    │
│  │  TUN 虚拟网卡         │                    │
│  │  IP: 10.0.0.1/24     │                    │
│  └────────┬─────────────┘                    │
│           │ read(fd, packet)                 │
│           ▼                                  │
│  ┌──────────────────────┐                    │
│  │  GAME++ TUN Reader   │                    │
│  │  解析 IP 层           │                    │
│  │  提取 TCP/UDP 载荷    │                    │
│  └────────┬─────────────┘                    │
│           │                                  │
│           ▼                                  │
│  ┌──────────────────────┐                    │
│  │  VLESS Tunnel        │                    │
│  │  → 远程节点 → 目标IP  │                    │
│  └────────┬─────────────┘                    │
│           │ 响应                                   │
│           ▼                                  │
│  ┌──────────────────────┐                    │
│  │  TUN Writer          │                    │
│  │  构造 IP 包写回 TUN   │                    │
│  └──────────────────────┘                    │
│                                              │
└─────────────────────────────────────────────┘

路由表配置示例（仅代理目标游戏服务器IP段）:

# 添加游戏服务器 IP 段走 TUN
route add 192.207.0.0/16 mask 255.255.0.0 10.0.0.1 metric 1
route add 159.153.0.0/16 mask 255.255.0.0 10.0.0.1 metric 1

# 其余流量走默认网关（直连）
```

### 5.5 节点测速方案

```
测速流程:
1. 获取节点列表
2. 并发 TCP Connect 到每个节点 (测量 TCP 握手 RTT)
3. 可选: 真实数据往返测试（通过已连接的 VLESS 隧道 ping 目标 IP）
4. 按延迟排序，标注丢包节点
5. 缓存结果（5分钟内有效）

并发策略: 使用 Tokio 并发测试，最多同时 20 个连接
超时设置: TCP 连接超时 3s，单节点测试超时 5s
```

---

## 6. 详细功能说明

### 6.1 订阅管理

**输入**: VLESS 订阅 URL (如 `https://pro.dl.214578.xyz/sub?token=xxx`)  
**处理流程**:

```
URL ─► HTTP GET ─► Base64 Decode ─► 逐行解析 VLESS URL ─► 节点列表
```

**VLESS URL 解析规则**:
```
格式: vless://UUID@IP:PORT?params#NAME

必选参数:
  - UUID: 用户标识
  - IP:PORT: 节点地址

可选参数解析:
  - security=tls  → TLS 开启
  - type=ws       → 传输协议 = WebSocket
  - host=xxx      → WebSocket Host 头
  - path=/xxx     → WebSocket 路径
  - sni=xxx       → TLS SNI
  - fp=chrome     → TLS 指纹
  - encryption=none → VLESS 加密模式
```

### 6.2 节点管理

- 按地区分组展示（HK/JP/SG/US/DE/NL）
- 显示：地区 | 地址 | 端口 | 延迟 | 丢包率 | 状态
- 收藏节点功能
- 手动添加/编辑节点
- 节点排序：延迟升序 / 地区分组 / 自定义

### 6.3 应用白名单

**Windows**:
- 扫描正在运行的进程列表（`sysinfo` crate）
- 用户选择需要加速的 .exe 进程
- 对于 TUN 模式：自动解析该进程连接的目标 IP，加入代理路由表
- 对于系统代理模式：部分应用需额外配置（环境变量 `HTTP_PROXY`）

**macOS**:
- 扫描 `/Applications/` 下的 .app Bundle
- 列出运行中的进程
- 同上规则管理

### 6.4 游戏预设库

内置配置文件，包含热门游戏的服务器 IP 段：

```json
{
  "games": [
    {
      "name": "Valorant",
      "regions": ["NA", "EU", "AP"],
      "ip_ranges": ["192.207.0.0/16", "159.153.0.0/16"],
      "recommended_nodes": ["US|官方优选", "DE|官方优选", "SG|官方优选"],
      "protocol": "udp"
    },
    {
      "name": "League of Legends",
      "regions": ["NA", "KR", "EUW"],
      "ip_ranges": ["104.160.0.0/16", "162.248.0.0/16"],
      "recommended_nodes": ["US|官方优选", "JP|官方优选", "DE|官方优选"],
      "protocol": "udp"
    }
  ]
}
```

---

## 7. 开发路线图

### Phase 1 — MVP (2-3 周)

**目标**: 可用的基本加速功能

```
□ 初始化 Tauri + Rust 项目骨架
□ 实现 VLESS URL 解析器
□ 实现订阅 URL 拉取 + Base64 解码
□ 实现本地 SOCKS5 代理服务器
□ 实现 VLESS over WS + TLS 客户端
□ 实现 TCP Ping 并发测速
□ 基础 UI：
  - 订阅地址输入 + 节点列表展示
  - 节点延迟显示
  - 一键连接/断开按钮
  - 连接状态指示
□ Windows 系统代理设置/取消
□ macOS 系统代理设置/取消
```

### Phase 2 — 核心体验 (2-3 周)

**目标**: 游戏可用 + 更好的体验

```
□ TUN 模式实现（Windows wintun + macOS utun）
□ 路由表管理（添加/删除游戏 IP 段路由）
□ 应用扫描 + 白名单选择 UI
□ 游戏预设库（5-10 款热门游戏）
□ 连接统计：实时流量、延迟监控图表
□ 节点智能推荐算法
□ 断线自动重连 + 节点自动切换
□ 系统托盘 + 最小化到托盘
```

### Phase 3 — 完善 (2-3 周)

**目标**: 成熟的加速器产品

```
□ Windows WFP 驱动（进程级代理，可选）
□ 游戏预设库扩展（20+ 游戏）
□ 配置导入/导出
□ 自定义代理规则编辑
□ 暗色/亮色主题
□ 开机自启
□ 自动更新
□ 多语言支持（中/英）
```

### Phase 4 — 进阶 (可选)

```
□ 自建中继节点支持
□ LAN 设备共享加速
□ 移动端配套 App
□ 社区规则市场
```

---

## 8. Rust Crate 依赖

### 8.1 核心依赖

```toml
[dependencies]
# Tauri 框架
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-shell = "2"
tauri-plugin-dialog = "2"

# 异步运行时
tokio = { version = "1", features = ["full"] }

# WebSocket 客户端
tokio-tungstenite = "0.24"
tungstenite = "0.24"

# TLS
rustls = "0.23"
rustls-native-certs = "0.8"
webpki-roots = "0.26"

# HTTP 客户端（订阅拉取）
reqwest = { version = "0.12", features = ["rustls-tls"], default-features = false }

# Base64 解码
base64 = "0.22"

# URL 解析
url = "2"

# 序列化
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# SOCKS5 服务端（可能需要自实现，或使用以下辅助）
socks5-server = "0.2"  # 社区 crate，但可能需要 fork/自实现

# TUN 设备
tun = "0.7"  # 跨平台 TUN 抽象

# 系统信息
sysinfo = "0.31"

# 系统代理设置
system-proxy = "0.3"  # 跨平台系统代理管理

# 日志
log = "0.4"
env_logger = "0.11"
tracing = "0.1"

# 错误处理
anyhow = "1"
thiserror = "1"

# 工具
uuid = { version = "1", features = ["v4"] }
chrono = "0.4"
parking_lot = "0.12"  # 更快的 Mutex

# 配置管理
directories = "5"  # 获取系统配置目录
toml = "0.8"
```

### 8.2 Windows 特有

```toml
[target.'cfg(windows)'.dependencies]
windows-sys = { version = "0.59", features = [
  "Win32_NetworkManagement_IpHelper",
  "Win32_Networking_WinSock",
  "Win32_System_Registry",
  "Win32_NetworkManagement_Ndis",
]} 
winreg = "0.52"          # 注册表操作
wintun = "0.4"           # WireGuard 的 wintun.dll 绑定
```

### 8.3 macOS 特有

```toml
[target.'cfg(target_os = "macos")'.dependencies]
system-configuration = "0.6"  # 系统代理设置
core-foundation = "0.9"
```

### 8.4 前端

```json
{
  "dependencies": {
    "@tauri-apps/api": "^2",
    "@tauri-apps/plugin-shell": "^2",
    "react": "^18",
    "react-dom": "^18",
    "recharts": "^2",        // 延迟图表
    "lucide-react": "^0.400", // 图标
    "tailwindcss": "^3",
    "zustand": "^4"          // 状态管理
  }
}
```

---

## 9. UI 设计概要

### 9.1 主窗口布局

```
┌──────────────────────────────────────────────┐
│  GAME++                          [_][□][×]  │
├──────────────────────────────────────────────┤
│                                              │
│  ┌──────────────────────────────────────┐    │
│  │  🟢 已连接 · HK|官方优选|60ms       │    │
│  │  ↑ 12.5 MB/s  ↓ 3.2 MB/s           │    │
│  │                          [断开连接]  │    │
│  └──────────────────────────────────────┘    │
│                                              │
│  ┌─ 加速模式 ──────────────────────────┐    │
│  │  ○ 系统代理  ● TUN 模式  ○ 仅代理   │    │
│  └──────────────────────────────────────┘    │
│                                              │
│  ┌─ 订阅管理 ──────────────────────────┐    │
│  │  [https://pro.dl.214578.xyz/sub...]  │    │
│  │  [更新订阅] [上次更新: 2分钟前]       │    │
│  └──────────────────────────────────────┘    │
│                                              │
│  ┌─ 节点列表 ─── [测速] [地区▼] ────────┐    │
│  │  ★ HK | 8.35.211.136:2083 | 60ms ●  │    │
│  │    HK | 8.35.211.157:2096 | 65ms ●  │    │
│  │    SG | 173.245.58.43:2087 | 100ms ●│    │
│  │    JP | 108.162.198.57:2087 | 110ms ●│   │
│  │    US | 103.31.4.187:443   | 243ms ●│    │
│  └──────────────────────────────────────┘    │
│                                              │
│  ┌─ 应用白名单 ────────────────────────┐    │
│  │  ☑ VALORANT-Win64-Shipping.exe     │    │
│  │  ☑ League of Legends.exe           │    │
│  │  ☐ EpicGamesLauncher.exe           │    │
│  │  [+ 添加应用]                       │    │
│  └──────────────────────────────────────┘    │
│                                              │
└──────────────────────────────────────────────┘
```

### 9.2 系统托盘菜单

```
┌──────────────────┐
│ 🟢 GAME++ 已连接  │
│ HK|官方优选 60ms  │
├──────────────────┤
│ 切换节点      ►  │
│ 加速模式      ►  │
├──────────────────┤
│ 显示主窗口       │
│ 断开连接         │
├──────────────────┤
│ 开机自启    ☑   │
│ 退出             │
└──────────────────┘
```

---

## 10. 风险评估

| 风险 | 影响 | 概率 | 缓解措施 |
|------|------|------|---------|
| VLESS 节点不稳定 | 游戏断线 | 高 | 自动重连 + 节点自动切换 + 连接池 |
| TUN 驱动兼容性 | Win上蓝屏/macOS上无法创建 | 中 | 充分测试、降级到系统代理模式 |
| 游戏反作弊检测 | 被封号 | 低(但致命) | 透明代理、不修改游戏进程 |
| TLS 指纹被墙 | 节点不可用 | 中 | 使用 Chrome 指纹、支持多指纹切换 |
| macOS 权限限制 | 无法创建 TUN | 中 | 引导用户授权、使用 Network Extension |
| Sub 订阅失效 | 节点全部不可用 | 低 | 支持多订阅源 |

---

## 附录

### A. 关键技术术语

| 术语 | 说明 |
|------|------|
| VLESS | V2Ray 轻量级协议，无内置加密，依赖 TLS |
| WebSocket | 将流量伪装成标准 HTTP WebSocket |
| TLS | 传输层加密，SNI 可伪装成普通网站 |
| TUN | 虚拟三层网卡，可读取原始 IP 包 |
| SOCKS5 | 标准代理协议，支持 TCP+UDP |
| WFP | Windows Filtering Platform，内核态网络过滤框架 |

### B. 项目结构建议

```
GAME++/
├── src-tauri/           # Rust 后端
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── src/
│   │   ├── main.rs          # 入口
│   │   ├── lib.rs           # Tauri 插件注册
│   │   ├── proxy/
│   │   │   ├── mod.rs
│   │   │   ├── vless.rs     # VLESS 客户端实现
│   │   │   ├── socks5.rs    # 本地 SOCKS5 服务端
│   │   │   ├── ws_tls.rs    # WebSocket + TLS 封装
│   │   │   └── tunnel.rs    # 隧道管理
│   │   ├── subscription/
│   │   │   ├── mod.rs
│   │   │   ├── parser.rs    # VLESS URL 解析
│   │   │   └── fetcher.rs   # 订阅拉取+解码
│   │   ├── tun/
│   │   │   ├── mod.rs
│   │   │   ├── windows.rs   # Win TUN 实现
│   │   │   └── macos.rs     # Mac TUN 实现
│   │   ├── route/
│   │   │   ├── mod.rs       # 路由表管理
│   │   │   └── rules.rs     # IP/CIDR 规则匹配
│   │   ├── monitor/
│   │   │   ├── mod.rs
│   │   │   ├── latency.rs   # 延迟测试
│   │   │   └── stats.rs     # 流量统计
│   │   ├── platform/
│   │   │   ├── mod.rs
│   │   │   ├── windows.rs   # Win 系统代理设置
│   │   │   └── macos.rs     # Mac 系统代理设置
│   │   └── commands.rs      # Tauri 命令（前端调用接口）
│   └── icons/
├── src/                  # React 前端
│   ├── App.tsx
│   ├── main.tsx
│   ├── components/
│   │   ├── NodeList.tsx
│   │   ├── SpeedTest.tsx
│   │   ├── AppWhitelist.tsx
│   │   ├── LatencyChart.tsx
│   │   └── StatusBar.tsx
│   ├── stores/
│   │   └── appStore.ts
│   └── styles/
│       └── globals.css
├── package.json
├── tsconfig.json
├── vite.config.ts
├── tailwind.config.js
└── PRD.md
```

### C. 参考资料

- VLESS 协议规范: https://xtls.github.io/
- V2Ray 文档: https://www.v2ray.com/
- Tauri 官方文档: https://v2.tauri.app/
- WireGuard wintun: https://www.wintun.net/
- SOCKS5 RFC 1928: https://www.ietf.org/rfc/rfc1928.txt
