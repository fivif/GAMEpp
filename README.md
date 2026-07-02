# GAME++

游戏网络加速器。指定进程 → 自动追踪 IP → 代理加速。

## 架构

```
游戏进程 → 网络监控 → 自动发现远端 IP → sing-box 代理隧道 → 游戏服务器
```

- **前端**: React + TypeScript + Tailwind CSS
- **后端**: Rust + Tauri 2.0
- **代理内核**: sing-box (VLESS / Trojan / Shadowsocks)

## 功能

- 扫描 Steam 游戏库，自动发现本地游戏
- 手动添加任意进程名进行加速
- 实时追踪进程网络连接，自动代理远端 IP
- 节点测速、地区筛选、智能选优
- 配置持久化，重启自动加速
- 支持 VLESS / Trojan / SS 订阅链接（URL 格式 + Clash YAML）

## 开发

```bash
# 安装依赖
brew install sing-box  # macOS
npm install

# 开发模式
npm run tauri dev

# 生产构建
npm run tauri build
```

## 订阅配置

支持标准代理订阅链接：
- `vless://` / `trojan://` / `ss://` 逐行格式
- Base64 编码订阅
- Clash YAML 格式

## 许可证

MIT
