import { useState, useEffect } from 'react';
import { useAppStore, InstalledGame } from './stores/appStore';
import { invoke } from '@tauri-apps/api/core';
import { Zap, Server, Settings, RefreshCw, Monitor, Plus, Play, Square, Gamepad2, Radio, X, Lock } from 'lucide-react';

const REGIONS = [
  { code: 'HK', name: '香港', desc: '低延迟' },
  { code: 'SG', name: '新加坡', desc: '低延迟' },
  { code: 'JP', name: '日本', desc: '中等延迟' },
  { code: 'US', name: '美国', desc: '高延迟' },
  { code: 'DE', name: '德国', desc: '高延迟' },
  { code: 'NL', name: '荷兰', desc: '高延迟' },
];

function HomeView() {
  const connected = useAppStore(s => s.connected);
  const selectedGame = useAppStore(s => s.selectedGame);
  const selectedRegion = useAppStore(s => s.selectedRegion);
  const setSelectedGame = useAppStore(s => s.setSelectedGame);
  const setSelectedRegion = useAppStore(s => s.setSelectedRegion);
  const nodes = useAppStore(s => s.nodes);
  const setNodes = useAppStore(s => s.setNodes);
  const setConnected = useAppStore(s => s.setConnected);
  const setCurrentNode = useAppStore(s => s.setCurrentNode);
  const localGames = useAppStore(s => s.localGames);
  const setLocalGames = useAppStore(s => s.setLocalGames);

  const [connecting, setConnecting] = useState(false);
  const [fetching, setFetching] = useState(false);
  const [processFound, setProcessFound] = useState(false);
  const [processPid, setProcessPid] = useState<number | null>(null);
  const [trackedIps, setTrackedIps] = useState<string[]>([]);
  const [customProcess, setCustomProcess] = useState('');

  const bestNode = [...nodes]
    .filter(n => n.region === selectedRegion && n.latency_ms !== null)
    .sort((a, b) => (a.latency_ms ?? 999) - (b.latency_ms ?? 999))[0];
  const regionNodes = nodes.filter(n => n.region === selectedRegion);
  const avgLatency = regionNodes.filter(n => n.latency_ms).length > 0
    ? Math.round(regionNodes.filter(n => n.latency_ms).reduce((s, n) => s + n.latency_ms!, 0) / regionNodes.filter(n => n.latency_ms).length)
    : null;

  // Detect game process
  useEffect(() => {
    if (!selectedGame) return;
    const detect = async () => {
      try {
        const pid = await invoke('find_game_process', { name: selectedGame.name });
        if (pid) {
          setProcessFound(true); setProcessPid(pid as number);
          const ips = await invoke('get_process_ips', { pid: pid as number });
          setTrackedIps(ips as string[]);
        } else {
          setProcessFound(false); setProcessPid(null); setTrackedIps([]);
        }
      } catch { setProcessFound(false); }
    };
    detect();
    const interval = setInterval(detect, 3000);
    return () => clearInterval(interval);
  }, [selectedGame]);

  const [scanDone, setScanDone] = useState(false);

  // Scan + load persisted config + auto-connect
  useEffect(() => {
    (async () => {
      // Load persisted config
      let savedUrl = '';
      let savedRegion = 'HK';
      let savedProcesses: string[] = [];
      try {
        const cfg = await invoke('load_persistent_config') as any;
        if (cfg?.subscription_url) { useAppStore.getState().setSubscriptionUrl(cfg.subscription_url); savedUrl = cfg.subscription_url; }
        if (cfg?.last_region) { useAppStore.getState().setSelectedRegion(cfg.last_region); savedRegion = cfg.last_region; }
        if (cfg?.custom_processes) savedProcesses = cfg.custom_processes;
      } catch {}

      // Scan local games
      try { const games = await invoke('scan_installed_games'); setLocalGames(games as InstalledGame[]); } catch {}
      setScanDone(true);

      // Restore saved custom processes
      if (savedProcesses.length > 0) {
        const saved: InstalledGame[] = savedProcesses.map(name => ({ name, exe_path: '', source: 'manual' }));
        const current = useAppStore.getState().localGames;
        setLocalGames([...saved, ...current.filter((g: InstalledGame) => g.source !== 'manual')]);
      }

      // Auto-fetch subscription
      if (savedUrl) {
        try {
          const list = await invoke('fetch_subscription', { url: savedUrl });
          useAppStore.getState().setNodes(list as any[]);
          useAppStore.getState().setSubscriptionUrl(savedUrl);

          // Auto speed test
          const results = await invoke('test_latency', { nodes: list });
          useAppStore.getState().setNodes(results as any[]);

          // Auto-select best node in saved region
          const allNodes = results as any[];
          const best = [...allNodes]
            .filter((n: any) => n.region === savedRegion && n.latency_ms !== null)
            .sort((a: any, b: any) => a.latency_ms - b.latency_ms)[0];

          // Auto-select Steam and connect
          useAppStore.getState().setSelectedGame({ name: 'Steam', exe_path: '', source: 'builtin' });
          if (best) {
            try {
              await invoke('start_proxy', { node: best });
              useAppStore.getState().setConnected(true);
              useAppStore.getState().setCurrentNode(best);
            } catch {}
          }
        } catch {}
      }
    })();
  }, []);

  const handleAddProcess = () => {
    if (!customProcess.trim()) return;
    const g: InstalledGame = { name: customProcess.trim(), exe_path: '', source: 'manual' };
    setLocalGames([...localGames.filter(x => x.name !== g.name), g]);
    setSelectedGame(g);
    setCustomProcess('');
  };

  const handleSpeedTest = async () => {
    if (nodes.length === 0) return;
    setFetching(true);
    try { const r = await invoke('test_latency', { nodes }); setNodes(r as any[]); } catch (e) { console.error(e); }
    finally { setFetching(false); }
  };

  const [errorMsg, setErrorMsg] = useState('');

  const handleStart = async () => {
    if (!bestNode) return;
    setConnecting(true); setErrorMsg('');
    try {
      await invoke('start_proxy', { node: bestNode });
      setConnected(true); setCurrentNode(bestNode);
    } catch (e: any) { setErrorMsg(String(e)); }
    finally { setConnecting(false); }
  };

  const handleStop = async () => {
    try { await invoke('stop_proxy'); setConnected(false); setCurrentNode(null); } catch (e) { console.error(e); }
  };

  return (
    <div className="flex-1 flex overflow-hidden">
      {/* Sidebar */}
      <div className="w-60 border-r border-white/[0.06] overflow-y-auto bg-[#0a0a0e] shrink-0 flex flex-col">
        {/* Permanent entries - always shown, cannot be deleted */}
        <div className="px-4 py-3 text-xs font-semibold text-zinc-500">常用</div>
        {['Steam', 'EpicGamesLauncher'].map(name => (
          <div key={name}
            className={`flex items-center ${selectedGame?.name === name ? 'bg-indigo-500/10 border-l-2 border-indigo-400' : 'border-l-2 border-transparent'}`}>
            <button
              onClick={() => setSelectedGame({ name, exe_path: '', source: 'builtin' })}
              className="flex-1 text-left px-4 py-2 text-sm text-zinc-400 hover:text-zinc-200 transition-colors">
              {name}
            </button>
          </div>
        ))}

        <div className="px-4 py-3 text-xs font-semibold text-zinc-500 mt-2">已添加</div>
        {!scanDone ? (
          <div className="px-4 py-2 text-xs text-zinc-700">扫描中...</div>
        ) : localGames.length === 0 ? (
          <div className="px-4 py-2 text-xs text-zinc-600">无</div>
        ) : (
          localGames.map(g => (
            <div key={g.exe_path || g.name}
              className={`flex items-center group ${selectedGame?.name === g.name ? 'bg-indigo-500/10 border-l-2 border-indigo-400' : 'border-l-2 border-transparent'}`}>
              <button
                onClick={() => setSelectedGame(g)}
                className="flex-1 text-left px-4 py-2 text-sm truncate text-zinc-400 hover:text-zinc-200 transition-colors">
                {g.name}
              </button>
              <button
                onClick={() => {
                  setLocalGames(localGames.filter(x => x.name !== g.name));
                  if (selectedGame?.name === g.name) setSelectedGame(null);
                }}
                className="px-2 py-2 text-zinc-600 hover:text-red-400 opacity-0 group-hover:opacity-100 transition-all">
                <X size={13} />
              </button>
            </div>
          ))
        )}

        {/* Manual add */}
        <div className="px-3 mt-auto pb-4 pt-3 border-t border-white/[0.04]">
          <div className="text-xs text-zinc-500 mb-2 px-1">手动添加</div>
          <div className="flex gap-1.5">
            <input value={customProcess}
              onChange={e => setCustomProcess(e.target.value)}
              onKeyDown={e => e.key === 'Enter' && handleAddProcess()}
              placeholder="进程名或 .exe"
              className="flex-1 px-3 py-1.5 bg-[#0a0a0f] border border-white/[0.06] rounded text-xs text-zinc-300 placeholder:text-zinc-600 outline-none focus:border-indigo-500/30" />
            <button onClick={handleAddProcess}
              className="px-3 py-1.5 bg-indigo-500/10 border border-indigo-500/20 rounded text-indigo-400 hover:bg-indigo-500/20">
              <Plus size={14} />
            </button>
          </div>
        </div>
      </div>

      {/* Main */}
      <div className="flex-1 overflow-auto">
        <div className="max-w-2xl mx-auto p-10">
          {/* Header */}
          <div className="text-center mb-8">
            <div className="mb-3">{selectedGame ? <Gamepad2 size={32} className="text-indigo-400 mx-auto" /> : <Radio size={32} className="text-zinc-600 mx-auto" />}</div>
            <h2 className="text-xl font-bold text-white">
              {selectedGame ? selectedGame.name : '选择程序'}
            </h2>
            <p className="text-sm text-zinc-500 mt-2">
              {selectedGame ? (processFound ? `进程运行中 · PID ${processPid}` : '等待程序启动...') : '从左侧列表选择程序，或手动添加进程名'}
            </p>

            {/* Region picker */}
            <div className="flex gap-2 justify-center mt-6 flex-wrap">
              {REGIONS.map(r => (
                <button key={r.code}
                  onClick={() => setSelectedRegion(r.code)}
                  className={`px-4 py-2 rounded-lg text-sm font-medium transition-all ${selectedRegion === r.code ? 'bg-indigo-500/20 text-indigo-400 ring-1 ring-indigo-500/30' : 'bg-white/[0.03] text-zinc-400 hover:bg-white/[0.06]'}`}>
                  {r.name}
                  <div className="text-[11px] opacity-50">{r.desc}</div>
                </button>
              ))}
            </div>
          </div>

          {/* IP tracking */}
          {selectedGame && processFound && (
            <div className="mb-6 p-4 rounded-xl bg-white/[0.02] border border-white/[0.05]">
              <div className="flex items-center gap-2 mb-3">
                <Monitor size={15} className="text-emerald-400" />
                <span className="text-sm font-medium text-zinc-300">网络追踪</span>
                <span className="text-xs text-zinc-500 ml-auto">{trackedIps.length} 个远端 IP</span>
              </div>
              {trackedIps.length > 0 ? (
                <div className="flex flex-wrap gap-1.5 max-h-24 overflow-y-auto">
                  {trackedIps.slice(0, 24).map(ip => (
                    <span key={ip} className="px-2 py-1 bg-indigo-500/10 border border-indigo-500/20 rounded text-xs text-indigo-400 font-mono">{ip}</span>
                  ))}
                  {trackedIps.length > 24 && <span className="text-xs text-zinc-600">+{trackedIps.length - 24} more</span>}
                </div>
              ) : (
                <div className="text-xs text-zinc-500">等待网络连接...</div>
              )}
            </div>
          )}

          {/* Stats */}
          <div className="mb-6 p-4 rounded-xl bg-white/[0.02] border border-white/[0.05] space-y-2">
            {[
              ['最优延迟', bestNode?.latency_ms ? `${bestNode.latency_ms}ms` : '--'],
              ['平均延迟', avgLatency ? `${avgLatency}ms` : '--'],
              ['可用节点', `${regionNodes.length}`],
            ].map(([label, val]) => (
              <div key={label} className="flex justify-between text-sm">
                <span className="text-zinc-500">{label}</span>
                <span className="font-mono text-zinc-200">{val}</span>
              </div>
            ))}
          </div>

          {/* Action */}
          <div className="space-y-3">
            {nodes.length === 0 ? (
              <div className="text-center text-sm text-zinc-500 py-4">
                请在顶部输入订阅地址并点击更新
              </div>
            ) : connected ? (
              <button onClick={handleStop} className="w-full py-3 rounded-xl bg-red-500/20 text-red-400 font-semibold border border-red-500/30 hover:bg-red-500/30 flex items-center justify-center gap-2">
                <Square size={15} />停止加速
              </button>
            ) : (
              <>
                <button onClick={handleStart} disabled={connecting || !selectedGame || !bestNode}
                  className="w-full py-3 rounded-xl bg-indigo-500 text-white font-semibold hover:bg-indigo-400 disabled:opacity-40 disabled:cursor-not-allowed flex justify-center items-center gap-2">
                  <Play size={16} />
                  {connecting ? '连接中...' : selectedGame ? `加速 ${selectedGame.name}` : '选择程序'}
                </button>
                <button onClick={handleSpeedTest} disabled={fetching}
                  className="w-full py-2 rounded-xl text-sm text-zinc-500 hover:text-zinc-300 flex justify-center items-center gap-2">
                  <RefreshCw size={13} className={fetching ? 'animate-spin' : ''} />测速选优
                </button>
              </>
            )}
            {errorMsg && <div className="mt-3 p-3 rounded-lg bg-red-500/10 border border-red-500/20 text-sm text-red-400 text-center break-all">{errorMsg}</div>}
          </div>
        </div>
      </div>
    </div>
  );
}

function SettingsView() {
  const subUrl = useAppStore(s => s.subscriptionUrl);
  const setSubUrl = useAppStore(s => s.setSubscriptionUrl);
  const setNodes = useAppStore(s => s.setNodes);
  const [pw, setPw] = useState('');
  const [unlocked, setUnlocked] = useState(false);
  const [url, setUrl] = useState('');
  const [msg, setMsg] = useState('');

  const handleUnlock = () => {
    if (pw === 'wwssaadd') { setUnlocked(true); setUrl(subUrl); }
    else setMsg('密码错误');
  };

  const handleSave = async () => {
    setSubUrl(url);
    setMsg('已保存');
    if (url.trim()) {
      try {
        const list = await invoke('fetch_subscription', { url: url.trim() });
        setNodes(list as any[]);
        setMsg('已保存并更新节点');
      } catch (e) { setMsg('保存成功，节点拉取失败: ' + e); }
    }
  };

  if (!unlocked) {
    return (
      <div className="flex-1 flex items-center justify-center p-8">
        <div className="w-72 text-center">
          <Lock size={24} className="text-zinc-600 mx-auto mb-4" />
          <h3 className="text-sm font-semibold text-white mb-4">设置</h3>
          <input type="password" value={pw} onChange={e => setPw(e.target.value)}
            onKeyDown={e => e.key === 'Enter' && handleUnlock()}
            placeholder="输入密码"
            className="w-full px-3 py-2 bg-[#0a0a0f] border border-white/[0.06] rounded text-sm text-zinc-300 placeholder:text-zinc-600 outline-none focus:border-indigo-500/30 mb-3" />
          <button onClick={handleUnlock}
            className="w-full py-2 rounded-lg bg-indigo-500 text-white text-sm font-medium hover:bg-indigo-400">
            解锁
          </button>
          {msg && <div className="text-xs text-red-400 mt-2">{msg}</div>}
        </div>
      </div>
    );
  }

  return (
    <div className="flex-1 flex items-center justify-center p-8">
      <div className="w-96 space-y-4">
        <h3 className="text-sm font-semibold text-white flex items-center gap-2">
          <Settings size={14} className="text-indigo-400" />设置
        </h3>
        <div className="bg-[#0c0c12] border border-white/[0.06] rounded-xl p-4 space-y-3">
          <div>
            <label className="text-xs text-zinc-500 block mb-1">订阅地址</label>
            <input value={url} onChange={e => setUrl(e.target.value)}
              placeholder="https://..."
              className="w-full px-3 py-2 bg-[#0a0a0f] border border-white/[0.06] rounded text-sm text-zinc-300 placeholder:text-zinc-600 outline-none focus:border-indigo-500/30" />
          </div>
          <button onClick={handleSave}
            className="w-full py-2 rounded-lg bg-indigo-500 text-white text-sm font-medium hover:bg-indigo-400">
            保存并更新节点
          </button>
          {msg && <div className="text-xs text-emerald-400 text-center">{msg}</div>}
        </div>
      </div>
    </div>
  );
}

function NodesView() {
  const state = useAppStore();
  const { nodes, connected, currentNode, nodeSearch: search, nodeRegionFilter: regionFilter, nodeSortBy: sortBy } = state;
  const { setNodeSearch: setSearch, setNodeRegionFilter: setRegionFilter, setNodeSortBy: setSortBy } = state;

  const regions = [...new Set(nodes.map(n => n.region))].sort();

  const filtered = nodes
    .filter(n => regionFilter === 'all' || n.region === regionFilter)
    .filter(n => !search || n.name.toLowerCase().includes(search.toLowerCase()) || n.address.includes(search))
    .sort((a, b) => sortBy === 'latency' ? (a.latency_ms ?? 9999) - (b.latency_ms ?? 9999) : a.name.localeCompare(b.name));

  return (
    <div className="flex-1 overflow-y-auto p-6">
      <div className="flex items-center gap-2 mb-4 flex-wrap">
        <h3 className="text-sm font-semibold text-white mr-2">节点列表 ({filtered.length}/{nodes.length})</h3>
        {/* Region filter buttons */}
        <button onClick={() => setRegionFilter('all')}
          className={`px-2 py-1 rounded text-xs ${regionFilter === 'all' ? 'bg-indigo-500/20 text-indigo-400' : 'bg-[#0a0a0f] text-zinc-500'}`}>全部</button>
        {regions.map(r => (
          <button key={r} onClick={() => setRegionFilter(r)}
            className={`px-2 py-1 rounded text-xs ${regionFilter === r ? 'bg-indigo-500/20 text-indigo-400' : 'bg-[#0a0a0f] text-zinc-500'}`}>{r}</button>
        ))}
        {/* Sort */}
        <button onClick={() => setSortBy(sortBy === 'latency' ? 'name' : 'latency')}
          className="px-2 py-1 rounded text-xs bg-[#0a0a0f] text-zinc-400 ml-2">
          {sortBy === 'latency' ? '↓延迟' : '↓名称'}
        </button>
        {/* Search */}
        <input value={search} onChange={e => setSearch(e.target.value)}
          placeholder="搜索..."
          className="px-2 py-1 bg-[#0a0a0f] border border-white/[0.06] rounded text-xs text-zinc-300 placeholder:text-zinc-600 outline-none focus:border-indigo-500/30 ml-2 w-40" />
      </div>

      <div className="space-y-0.5">
        {filtered.slice(0, 300).map(n => {
          const isCurrent = connected && currentNode?.name === n.name;
          const ms = n.latency_ms;
          return (
            <div key={n.name} className={`flex items-center gap-3 px-3 py-2 rounded-lg text-sm ${isCurrent ? 'bg-emerald-500/10 border border-emerald-500/20' : 'hover:bg-white/[0.03] border border-transparent'}`}>
              <span className="w-8 h-5 rounded bg-white/[0.06] flex items-center justify-center text-[10px] font-bold text-zinc-500 uppercase shrink-0">{n.region}</span>
              <span className="flex-1 truncate text-zinc-300">{n.name}</span>
              <span className="text-zinc-600 text-xs shrink-0">{n.address}:{n.port}</span>
              <span className={`text-xs font-mono w-14 text-right shrink-0 ${ms === null ? 'text-zinc-700' : ms < 100 ? 'text-emerald-400' : ms < 200 ? 'text-amber-400' : 'text-red-400'}`}>
                {ms !== null ? `${ms}ms` : '--'}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

export default function App() {
  const currentView = useAppStore(s => s.currentView);
  const setCurrentView = useAppStore(s => s.setCurrentView);
  const connected = useAppStore(s => s.connected);
  const currentNode = useAppStore(s => s.currentNode);
  const subscriptionUrl = useAppStore(s => s.subscriptionUrl);
  const setSubscriptionUrl = useAppStore(s => s.setSubscriptionUrl);
  const setNodes = useAppStore(s => s.setNodes);
  const selectedRegion = useAppStore(s => s.selectedRegion);
  const localGames = useAppStore(s => s.localGames);

  const [subInput, setSubInput] = useState(subscriptionUrl);
  const [fetching, setFetching] = useState(false);

  // Auto-save config when state changes
  useEffect(() => {
    const timer = setTimeout(async () => {
      try {
        await invoke('save_persistent_config', { config: {
          subscription_url: subscriptionUrl,
          last_region: selectedRegion,
          custom_processes: localGames.filter(g => g.source === 'manual').map(g => g.name),
          proxy_mode: 'system', socks_port: 1080, http_port: 1081,
          selected_node_id: null, whitelisted_apps: [], auto_connect: false,
        }});
      } catch {}
    }, 1000);
    return () => clearTimeout(timer);
  }, [subscriptionUrl, selectedRegion, localGames]);

  const handleFetchSub = async () => {
    if (!subInput.trim()) return;
    setFetching(true);
    try {
      const list = await invoke('fetch_subscription', { url: subInput.trim() });
      setNodes(list as any[]);
      setSubscriptionUrl(subInput.trim());
    } catch (e) { alert('获取失败: ' + e); }
    finally { setFetching(false); }
  };

  return (
    <div className="h-screen flex flex-col bg-[#08080c] text-zinc-200 overflow-hidden">
      <header className="h-11 border-b border-white/[0.06] flex items-center justify-between px-4 shrink-0 bg-[#0c0c12]">
        <div className="flex items-center gap-2.5">
          <div className="flex items-center gap-0.5">
            {([['home', Zap], ['nodes', Server], ['settings', Settings]] as const).map(([id, Icon]) => (
              <button key={id} onClick={() => setCurrentView(id)}
                className={`p-1 rounded transition-colors ${currentView === id ? 'bg-white/[0.06] text-zinc-200' : 'text-zinc-600 hover:text-zinc-400'}`}>
                <Icon size={14} />
              </button>
            ))}
          </div>
        </div>
        <div className="flex items-center gap-2">
          <div className={`flex items-center gap-1 px-2 py-0.5 rounded text-[10px] font-medium ml-1 ${connected ? 'bg-emerald-500/10 text-emerald-400' : 'bg-zinc-800 text-zinc-600'}`}>
            <div className={`w-1.5 h-1.5 rounded-full ${connected ? 'bg-emerald-400' : 'bg-zinc-600'}`} />
            {connected ? `${currentNode?.latency_ms ?? '?'}ms` : '离线'}
          </div>
        </div>
      </header>
      {currentView === 'nodes' ? <NodesView /> : currentView === 'settings' ? <SettingsView /> : <HomeView />}
    </div>
  );
}
