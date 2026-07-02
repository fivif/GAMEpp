import { create } from 'zustand';

export interface ProxyNode {
  name: string; protocol: string; address: string; port: number;
  uuid: string; security: string; transport: string; host: string;
  path: string; encryption: string; fingerprint: string;
  region: string; latency_ms: number | null; is_connected: boolean; raw_url: string;
}

export interface InstalledGame { name: string; exe_path: string; source: string; }

interface AppState {
  connected: boolean; currentNode: ProxyNode | null;
  selectedGame: InstalledGame | null; selectedRegion: string;
  subscriptionUrl: string; nodes: ProxyNode[];
  currentView: 'home' | 'nodes' | 'settings'; localGames: InstalledGame[];
  nodeSearch: string; nodeRegionFilter: string; nodeSortBy: 'latency' | 'name';
  setConnected: (v: boolean) => void; setCurrentNode: (n: ProxyNode | null) => void;
  setSelectedGame: (g: InstalledGame | null) => void; setSelectedRegion: (c: string) => void;
  setSubscriptionUrl: (u: string) => void; setNodes: (n: ProxyNode[]) => void;
  setCurrentView: (v: 'home' | 'nodes' | 'settings') => void;
  setLocalGames: (g: InstalledGame[]) => void;
  setNodeSearch: (v: string) => void; setNodeRegionFilter: (v: string) => void;
  setNodeSortBy: (v: 'latency' | 'name') => void;
}

export const useAppStore = create<AppState>((set) => ({
  connected: false, currentNode: null, selectedGame: null, selectedRegion: 'HK',
  subscriptionUrl: '', nodes: [], currentView: 'home', localGames: [],
  nodeSearch: '', nodeRegionFilter: 'all', nodeSortBy: 'latency',
  setConnected: (c) => set({ connected: c }),
  setCurrentNode: (n) => set({ currentNode: n }),
  setSelectedGame: (g) => set({ selectedGame: g }),
  setSelectedRegion: (r) => set({ selectedRegion: r }),
  setSubscriptionUrl: (u) => set({ subscriptionUrl: u }),
  setNodes: (n) => set({ nodes: n }),
  setCurrentView: (v) => set({ currentView: v }),
  setLocalGames: (g) => set({ localGames: g }),
  setNodeSearch: (v) => set({ nodeSearch: v }),
  setNodeRegionFilter: (v) => set({ nodeRegionFilter: v }),
  setNodeSortBy: (v) => set({ nodeSortBy: v }),
}));
