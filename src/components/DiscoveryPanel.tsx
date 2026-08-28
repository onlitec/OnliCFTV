import React, { useState, useEffect, useMemo } from 'react';
import {
  Search,
  RefreshCw,
  Plus,
  Check,
  CheckCircle2,
  AlertTriangle,
  Eye,
  Loader2,
  Camera,
  Layers,
  Server,
  PhoneCall,
  Car,
  Compass,
  Thermometer,
  Boxes,
  Network,
  Copy,
  Info,
  Download,
  ChevronUp,
  ChevronDown,
  ChevronsUpDown,
} from 'lucide-react';

type SortColumn = 'ip' | 'type' | 'model' | 'status' | 'confidence';

// Compares dotted-decimal IPs octet-by-octet as numbers (e.g. "10.0.0.2" < "10.0.0.10"),
// instead of a lexicographic string compare that would get that pair backwards.
function compareIpAddresses(a: string, b: string): number {
  const partsA = a.split('.').map(Number);
  const partsB = b.split('.').map(Number);
  for (let i = 0; i < 4; i++) {
    const diff = (partsA[i] ?? 0) - (partsB[i] ?? 0);
    if (diff !== 0) return diff;
  }
  return 0;
}

interface SortableHeaderProps {
  label: string;
  column: SortColumn;
  sortColumn: SortColumn | null;
  sortDirection: 'asc' | 'desc';
  onSort: (column: SortColumn) => void;
}

const SortableHeader: React.FC<SortableHeaderProps> = ({ label, column, sortColumn, sortDirection, onSort }) => {
  const isActive = sortColumn === column;
  return (
    <th className="px-3 py-2.5">
      <button
        type="button"
        onClick={() => onSort(column)}
        className={`flex items-center gap-1 uppercase text-[10px] tracking-wider font-semibold ${
          isActive ? 'text-sky-600 dark:text-sky-400' : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white'
        }`}
        title={`Ordenar por ${label}`}
      >
        {label}
        {isActive ? (
          sortDirection === 'asc' ? <ChevronUp className="h-3 w-3" /> : <ChevronDown className="h-3 w-3" />
        ) : (
          <ChevronsUpDown className="h-3 w-3 opacity-40" />
        )}
      </button>
    </th>
  );
};
import type {
  DiscoveredDevice,
  DeviceType,
  NetworkInterfaceInfo,
  BatchCreateCamerasInput,
  CreateCameraInput,
} from '@/types';
import { api } from '@/services/api';
import { QuickViewerModal } from '@/components/QuickViewerModal';

interface DiscoveryPanelProps {
  discoveredDevices: DiscoveredDevice[];
  isScanning: boolean;
  onRefreshScan: (interfaceName?: string) => void;
  onAdded: () => void;
  onAddSingle: (prefill: CreateCameraInput) => void;
  onOpenManualAdd?: () => void;
}

export const DiscoveryPanel: React.FC<DiscoveryPanelProps> = ({
  discoveredDevices,
  isScanning,
  onRefreshScan,
  onAdded,
  onAddSingle,
  onOpenManualAdd,
}) => {
  const [interfaces, setInterfaces] = useState<NetworkInterfaceInfo[]>([]);
  const [selectedInterface, setSelectedInterface] = useState<string>('');
  const [activeFilter, setActiveFilter] = useState<string>('all');
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedIps, setSelectedIps] = useState<Set<string>>(new Set());
  const [copiedText, setCopiedText] = useState<string | null>(null);
  const [expandedEvidencesIp, setExpandedEvidencesIp] = useState<string | null>(null);

  // Column sorting
  const [sortColumn, setSortColumn] = useState<SortColumn | null>(null);
  const [sortDirection, setSortDirection] = useState<'asc' | 'desc'>('asc');

  const handleSort = (column: SortColumn) => {
    if (sortColumn === column) {
      setSortDirection((d) => (d === 'asc' ? 'desc' : 'asc'));
    } else {
      setSortColumn(column);
      setSortDirection('asc');
    }
  };

  // Quick Viewer Modal state
  const [quickViewDevice, setQuickViewDevice] = useState<DiscoveredDevice | null>(null);

  // Batch modal state
  const [isBatchOpen, setIsBatchOpen] = useState(false);
  const [batchUsername, setBatchUsername] = useState('admin');
  const [batchPassword, setBatchPassword] = useState('');
  const [showPassword, setShowPassword] = useState(false);
  const [batchProfile, setBatchProfile] = useState<'main' | 'sub'>('main');
  const [isSavingBatch, setIsSavingBatch] = useState(false);
  const [batchError, setBatchError] = useState<string | null>(null);

  // Load Network Interfaces on mount
  useEffect(() => {
    api.getNetworkInterfaces().then((ifaces) => {
      setInterfaces(ifaces);
      const def = ifaces.find((i) => i.is_default) || ifaces[0];
      if (def) {
        setSelectedInterface(def.id);
      }
    }).catch(console.error);
  }, []);

  const currentIfaceInfo = useMemo(() => {
    return (interfaces || []).find((i) => i.id === selectedInterface) || (interfaces || [])[0];
  }, [interfaces, selectedInterface]);

  // Filter Counts
  const counts = useMemo(() => {
    const map: Record<string, number> = {
      all: (discoveredDevices || []).length,
      ip_camera: 0,
      intercom: 0,
      nvr: 0,
      server: 0,
      switch: 0,
      router: 0,
      traffic_lpr: 0,
      ptz: 0,
      thermal: 0,
      with_issues: 0,
      other: 0,
    };
    for (const d of (discoveredDevices || [])) {
      if (map[d.device_type] !== undefined) {
        map[d.device_type]++;
      } else {
        map.other++;
      }
      if (d.issues && d.issues.length > 0) {
        map.with_issues++;
      }
    }
    return map;
  }, [discoveredDevices]);

  // Filtered devices list
  const filteredDevices = useMemo(() => {
    const filtered = (discoveredDevices || []).filter((d) => {
      let matchesType = true;
      if (activeFilter === 'with_issues') {
        matchesType = d.issues && d.issues.length > 0;
      } else if (activeFilter !== 'all') {
        matchesType = d.device_type === activeFilter;
      }

      const q = searchQuery.toLowerCase().trim();
      const matchesQuery =
        !q ||
        d.name.toLowerCase().includes(q) ||
        d.ip.toLowerCase().includes(q) ||
        (d.mac && d.mac.toLowerCase().includes(q)) ||
        d.hardware_model.toLowerCase().includes(q) ||
        d.brand.toLowerCase().includes(q) ||
        (d.serial_number && d.serial_number.toLowerCase().includes(q)) ||
        d.device_type_label.toLowerCase().includes(q);

      return matchesType && matchesQuery;
    });

    if (!sortColumn) return filtered;

    const sorted = [...filtered].sort((a, b) => {
      let cmp = 0;
      switch (sortColumn) {
        case 'ip':
          cmp = compareIpAddresses(a.ip, b.ip);
          break;
        case 'type':
          cmp = a.device_type_label.localeCompare(b.device_type_label);
          break;
        case 'model':
          cmp = a.hardware_model.localeCompare(b.hardware_model);
          break;
        case 'status':
          cmp = (a.activation_status || '').localeCompare(b.activation_status || '');
          break;
        case 'confidence':
          cmp = a.confidence_score - b.confidence_score;
          break;
      }
      return sortDirection === 'asc' ? cmp : -cmp;
    });
    return sorted;
  }, [discoveredDevices, activeFilter, searchQuery, sortColumn, sortDirection]);

  const unaddedInFilter = filteredDevices.filter((d) => !d.is_already_added);

  const toggleSelect = (ip: string) => {
    const next = new Set(selectedIps);
    if (next.has(ip)) {
      next.delete(ip);
    } else {
      next.add(ip);
    }
    setSelectedIps(next);
  };

  const toggleSelectAllFiltered = () => {
    const unaddedIps = unaddedInFilter.map((d) => d.ip);
    const allSelected = unaddedIps.length > 0 && unaddedIps.every((ip) => selectedIps.has(ip));

    const next = new Set(selectedIps);
    if (allSelected) {
      unaddedIps.forEach((ip) => next.delete(ip));
    } else {
      unaddedIps.forEach((ip) => next.add(ip));
    }
    setSelectedIps(next);
  };

  const handleCopy = (text: string) => {
    navigator.clipboard.writeText(text);
    setCopiedText(text);
    setTimeout(() => setCopiedText(null), 2000);
  };

  const handleExportCsv = () => {
    const headers = ['Nº', 'IP', 'Tipo', 'Fabricante', 'Modelo', 'MAC', 'Status', 'Portas', 'Confiança'];
    const rows = filteredDevices.map((d, idx) => [
      idx + 1,
      d.ip,
      d.device_type_label,
      d.brand,
      d.hardware_model,
      d.mac || '',
      d.activation_status,
      d.protocols.join(';'),
      `${d.confidence_score}%`,
    ]);

    const csvContent = [headers.join(','), ...rows.map((r) => r.map((c) => `"${c}"`).join(','))].join('\n');
    const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.setAttribute('href', url);
    link.setAttribute('download', `onliview_dispositivos_${Date.now()}.csv`);
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
  };

  const handleConfirmBatch = async (e: React.FormEvent) => {
    e.preventDefault();
    if (selectedIps.size === 0) return;

    setIsSavingBatch(true);
    setBatchError(null);

    try {
      const selectedDevices = (discoveredDevices || []).filter((d) => selectedIps.has(d.ip));
      const input: BatchCreateCamerasInput = {
        devices: selectedDevices.map((d) => ({
          name: d.name,
          host: d.ip,
          rtsp_port: d.rtsp_port || 554,
          http_port: d.http_port || 80,
          // A Descoberta ja classificou NVR/DVR: registra com o tipo certo para
          // que o gravador apareca na Verificacao de Gravacoes.
          device_type: d.device_type,
        })),
        username: batchUsername.trim() || 'admin',
        password: batchPassword || undefined,
        stream_profile: batchProfile,
      };

      await api.createCamerasBatch(input);
      setIsBatchOpen(false);
      setSelectedIps(new Set());
      onAdded();
    } catch (err: any) {
      setBatchError(err?.toString() || 'Erro ao adicionar câmeras em lote');
    } finally {
      setIsSavingBatch(false);
    }
  };

  const getTypeIcon = (type: DeviceType) => {
    switch (type) {
      case 'intercom':
        return <PhoneCall className="h-3.5 w-3.5 text-purple-600 dark:text-purple-400" />;
      case 'traffic_lpr':
        return <Car className="h-3.5 w-3.5 text-amber-600 dark:text-amber-400" />;
      case 'nvr':
      case 'dvr':
        return <Server className="h-3.5 w-3.5 text-blue-600 dark:text-blue-400" />;
      case 'ptz':
        return <Compass className="h-3.5 w-3.5 text-cyan-600 dark:text-cyan-400" />;
      case 'thermal':
        return <Thermometer className="h-3.5 w-3.5 text-rose-600 dark:text-rose-400" />;
      case 'server':
        return <Server className="h-3.5 w-3.5 text-orange-600 dark:text-orange-400" />;
      case 'switch':
        return <Network className="h-3.5 w-3.5 text-indigo-600 dark:text-indigo-400" />;
      case 'router':
        return <Network className="h-3.5 w-3.5 text-cyan-600 dark:text-cyan-400" />;
      case 'ip_camera':
        return <Camera className="h-3.5 w-3.5 text-emerald-600 dark:text-emerald-400" />;
      default:
        return <Boxes className="h-3.5 w-3.5 text-slate-500 dark:text-slate-400" />;
    }
  };

  const getTypeBadgeClass = (type: DeviceType) => {
    switch (type) {
      case 'intercom':
        return 'bg-purple-50 dark:bg-purple-500/15 text-purple-700 dark:text-purple-300 border-purple-200 dark:border-purple-500/30';
      case 'traffic_lpr':
        return 'bg-amber-50 dark:bg-amber-500/15 text-amber-700 dark:text-amber-300 border-amber-200 dark:border-amber-500/30';
      case 'nvr':
      case 'dvr':
        return 'bg-blue-50 dark:bg-blue-500/15 text-blue-700 dark:text-blue-300 border-blue-200 dark:border-blue-500/30';
      case 'ptz':
        return 'bg-cyan-50 dark:bg-cyan-500/15 text-cyan-700 dark:text-cyan-300 border-cyan-200 dark:border-cyan-500/30';
      case 'thermal':
        return 'bg-rose-50 dark:bg-rose-500/15 text-rose-700 dark:text-rose-300 border-rose-200 dark:border-rose-500/30';
      case 'server':
        return 'bg-orange-50 dark:bg-orange-500/15 text-orange-700 dark:text-orange-300 border-orange-200 dark:border-orange-500/30';
      case 'switch':
        return 'bg-indigo-50 dark:bg-indigo-500/15 text-indigo-700 dark:text-indigo-300 border-indigo-200 dark:border-indigo-500/30';
      case 'router':
        return 'bg-cyan-50 dark:bg-cyan-500/15 text-cyan-700 dark:text-cyan-300 border-cyan-200 dark:border-cyan-500/30';
      case 'ip_camera':
        return 'bg-emerald-50 dark:bg-emerald-500/15 text-emerald-700 dark:text-emerald-300 border-emerald-200 dark:border-emerald-500/30';
      default:
        return 'bg-slate-100 dark:bg-slate-800 text-slate-700 dark:text-slate-400 border-slate-200 dark:border-slate-700';
    }
  };

  return (
    <div className="bg-white dark:bg-slate-950 border border-slate-200 dark:border-slate-800 rounded-xl overflow-hidden shadow-md dark:shadow-2xl flex flex-col h-[calc(100vh-5.5rem)] transition-colors">
      {/* 1. TOP HEADER TOOLBAR — Commissioning Controls */}
      <div className="px-5 py-2.5 bg-white dark:bg-slate-900 border-b border-slate-200 dark:border-slate-800 flex flex-wrap items-center justify-between gap-3 shrink-0">
        {/* Left: Sub-network tabs and Device Count */}
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-1.5 bg-slate-50 dark:bg-slate-950 border border-slate-200 dark:border-slate-800 rounded-lg p-1">
            <button
              className="px-3 py-1 rounded bg-sky-600 hover:bg-sky-500 text-white text-xs font-bold shadow flex items-center gap-1.5 transition"
            >
              <span>Sub-rede Atual ({currentIfaceInfo?.name || 'Local'})</span>
            </button>

            {/* NIC Dropdown */}
            <select
              value={selectedInterface}
              onChange={(e) => setSelectedInterface(e.target.value)}
              className="bg-transparent text-slate-700 dark:text-slate-300 text-xs font-semibold focus:outline-none cursor-pointer px-2 py-1"
            >
              {(interfaces || []).map((iface) => (
                <option key={iface.id} value={iface.id} className="bg-white dark:bg-slate-900 text-slate-900 dark:text-white">
                  {iface.name}: {iface.ip}/{iface.netmask}
                </option>
              ))}
            </select>
          </div>

          <div className="text-xs font-bold text-slate-700 dark:text-slate-300 flex items-center gap-1.5 font-mono">
            <span>Pesquisar:</span>
            <span className="px-2 py-0.5 rounded-full bg-sky-50 dark:bg-sky-500/20 text-sky-600 dark:text-sky-300 border border-sky-200 dark:border-sky-500/30">
              {(discoveredDevices || []).length} encontrados
            </span>
          </div>
        </div>

        {/* Right: Actions, Search & Buttons */}
        <div className="flex items-center gap-2.5">

          {/* Quick Search */}
          <div className="relative w-48 sm:w-56">
            <Search className="h-3.5 w-3.5 absolute left-2.5 top-1/2 -translate-y-1/2 text-slate-400" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="IP, MAC, modelo, tipo..."
              className="w-full bg-slate-50 dark:bg-slate-950 border border-slate-200 dark:border-slate-800 rounded-lg pl-8 pr-3 py-1.5 text-xs text-slate-900 dark:text-white placeholder-slate-400 dark:placeholder-slate-500 focus:outline-none focus:border-sky-500 font-sans"
            />
          </div>

          {/* Refresh / Scan */}
          <button
            onClick={() => onRefreshScan(selectedInterface || undefined)}
            disabled={isScanning}
            className="px-3 py-1.5 rounded-lg bg-sky-600 hover:bg-sky-500 text-white text-xs font-bold flex items-center gap-1.5 transition disabled:opacity-50 shadow-sm"
            title="Atualizar busca na rede"
          >
            <RefreshCw className={`h-3.5 w-3.5 ${isScanning ? 'animate-spin' : ''}`} />
            <span>{isScanning ? 'Varrendo...' : 'Atualizar'}</span>
          </button>

          {/* Manual Add */}
          {onOpenManualAdd && (
            <button
              onClick={onOpenManualAdd}
              className="px-3 py-1.5 rounded-lg bg-slate-100 dark:bg-slate-800 hover:bg-slate-200 dark:hover:bg-slate-700 text-slate-700 dark:text-slate-200 border border-slate-200 dark:border-slate-700 text-xs font-semibold flex items-center gap-1.5 transition shadow-sm"
            >
              <Plus className="h-3.5 w-3.5" />
              <span>Adicionar Manual</span>
            </button>
          )}

          {/* Export CSV */}
          <button
            onClick={handleExportCsv}
            className="p-1.5 rounded-lg bg-slate-100 dark:bg-slate-950 hover:bg-slate-200 dark:hover:bg-slate-800 text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white border border-slate-200 dark:border-slate-800 transition shadow-sm"
            title="Exportar lista em CSV"
          >
            <Download className="h-4 w-4" />
          </button>
        </div>
      </div>

      {/* 2. CATEGORY FILTER PILLS BAR */}
      <div className="px-5 py-2 bg-slate-50/80 dark:bg-slate-950/80 border-b border-slate-200 dark:border-slate-800/80 flex items-center gap-1.5 overflow-x-auto scrollbar-none text-xs shrink-0">
        <button
          onClick={() => setActiveFilter('all')}
          className={`px-2.5 py-1 rounded-lg font-medium transition flex items-center gap-1.5 shrink-0 ${
            activeFilter === 'all'
              ? 'bg-sky-600 text-white font-bold shadow'
              : 'bg-white dark:bg-slate-900 text-slate-700 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 border border-slate-200 dark:border-slate-800'
          }`}
        >
          <Boxes className="h-3.5 w-3.5" />
          <span>Todos</span>
          <span className="px-1.5 py-0.2 rounded-full text-[10px] bg-slate-200 dark:bg-black/40 font-mono">{counts.all}</span>
        </button>

        {counts.ip_camera > 0 && (
          <button
            onClick={() => setActiveFilter('ip_camera')}
            className={`px-2.5 py-1 rounded-lg font-medium transition flex items-center gap-1.5 shrink-0 ${
              activeFilter === 'ip_camera'
                ? 'bg-emerald-600 text-white font-bold shadow'
                : 'bg-white dark:bg-slate-900 text-slate-700 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 border border-slate-200 dark:border-slate-800'
            }`}
          >
            <Camera className="h-3.5 w-3.5 text-emerald-600 dark:text-emerald-400" />
            <span>Câmeras IP</span>
            <span className="px-1.5 py-0.2 rounded-full text-[10px] bg-slate-200 dark:bg-black/40 font-mono">{counts.ip_camera}</span>
          </button>
        )}

        {counts.intercom > 0 && (
          <button
            onClick={() => setActiveFilter('intercom')}
            className={`px-2.5 py-1 rounded-lg font-medium transition flex items-center gap-1.5 shrink-0 ${
              activeFilter === 'intercom'
                ? 'bg-purple-600 text-white font-bold shadow'
                : 'bg-white dark:bg-slate-900 text-slate-700 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 border border-slate-200 dark:border-slate-800'
            }`}
          >
            <PhoneCall className="h-3.5 w-3.5 text-purple-600 dark:text-purple-400" />
            <span>Videoporteiros</span>
            <span className="px-1.5 py-0.2 rounded-full text-[10px] bg-slate-200 dark:bg-black/40 font-mono">{counts.intercom}</span>
          </button>
        )}

        {counts.nvr > 0 && (
          <button
            onClick={() => setActiveFilter('nvr')}
            className={`px-2.5 py-1 rounded-lg font-medium transition flex items-center gap-1.5 shrink-0 ${
              activeFilter === 'nvr'
                ? 'bg-blue-600 text-white font-bold shadow'
                : 'bg-white dark:bg-slate-900 text-slate-700 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 border border-slate-200 dark:border-slate-800'
            }`}
          >
            <Server className="h-3.5 w-3.5 text-blue-600 dark:text-blue-400" />
            <span>NVRs / Gravadores</span>
            <span className="px-1.5 py-0.2 rounded-full text-[10px] bg-slate-200 dark:bg-black/40 font-mono">{counts.nvr}</span>
          </button>
        )}

        {counts.server > 0 && (
          <button
            onClick={() => setActiveFilter('server')}
            className={`px-2.5 py-1 rounded-lg font-medium transition flex items-center gap-1.5 shrink-0 ${
              activeFilter === 'server'
                ? 'bg-orange-600 text-white font-bold shadow'
                : 'bg-white dark:bg-slate-900 text-slate-700 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 border border-slate-200 dark:border-slate-800'
            }`}
          >
            <Server className="h-3.5 w-3.5 text-orange-600 dark:text-orange-400" />
            <span>Servidores</span>
            <span className="px-1.5 py-0.2 rounded-full text-[10px] bg-slate-200 dark:bg-black/40 font-mono">{counts.server}</span>
          </button>
        )}

        {counts.switch > 0 && (
          <button
            onClick={() => setActiveFilter('switch')}
            className={`px-2.5 py-1 rounded-lg font-medium transition flex items-center gap-1.5 shrink-0 ${
              activeFilter === 'switch'
                ? 'bg-indigo-600 text-white font-bold shadow'
                : 'bg-white dark:bg-slate-900 text-slate-700 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 border border-slate-200 dark:border-slate-800'
            }`}
          >
            <Network className="h-3.5 w-3.5 text-indigo-600 dark:text-indigo-400" />
            <span>Switches</span>
            <span className="px-1.5 py-0.2 rounded-full text-[10px] bg-slate-200 dark:bg-black/40 font-mono">{counts.switch}</span>
          </button>
        )}

        {counts.router > 0 && (
          <button
            onClick={() => setActiveFilter('router')}
            className={`px-2.5 py-1 rounded-lg font-medium transition flex items-center gap-1.5 shrink-0 ${
              activeFilter === 'router'
                ? 'bg-cyan-600 text-white font-bold shadow'
                : 'bg-white dark:bg-slate-900 text-slate-700 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 border border-slate-200 dark:border-slate-800'
            }`}
          >
            <Network className="h-3.5 w-3.5 text-cyan-600 dark:text-cyan-400" />
            <span>Roteadores</span>
            <span className="px-1.5 py-0.2 rounded-full text-[10px] bg-slate-200 dark:bg-black/40 font-mono">{counts.router}</span>
          </button>
        )}

        {counts.other > 0 && (
          <button
            onClick={() => setActiveFilter('other')}
            className={`px-2.5 py-1 rounded-lg font-medium transition flex items-center gap-1.5 shrink-0 ${
              activeFilter === 'other'
                ? 'bg-slate-700 text-white font-bold shadow'
                : 'bg-white dark:bg-slate-900 text-slate-700 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 border border-slate-200 dark:border-slate-800'
            }`}
          >
            <Boxes className="h-3.5 w-3.5 text-slate-500 dark:text-slate-400" />
            <span>Não Identificados</span>
            <span className="px-1.5 py-0.2 rounded-full text-[10px] bg-slate-200 dark:bg-black/40 font-mono">{counts.other}</span>
          </button>
        )}

        {counts.with_issues > 0 && (
          <button
            onClick={() => setActiveFilter('with_issues')}
            className={`px-2.5 py-1 rounded-lg font-medium transition flex items-center gap-1.5 shrink-0 ${
              activeFilter === 'with_issues'
                ? 'bg-amber-600 text-white font-bold shadow'
                : 'bg-white dark:bg-slate-900 text-amber-600 dark:text-amber-400 hover:text-amber-800 dark:hover:text-amber-200 hover:bg-slate-100 dark:hover:bg-slate-800 border border-amber-200 dark:border-amber-500/30'
            }`}
          >
            <AlertTriangle className="h-3.5 w-3.5 text-amber-600 dark:text-amber-400" />
            <span>Com Alertas</span>
            <span className="px-1.5 py-0.2 rounded-full text-[10px] bg-slate-200 dark:bg-black/40 font-mono">{counts.with_issues}</span>
          </button>
        )}
      </div>

      {/* 3. HIGH DENSITY TABLE AREA */}
      <div className="flex-1 min-h-0 overflow-y-auto">
        {isScanning && (discoveredDevices || []).length === 0 ? (
          <div className="py-20 text-center space-y-3">
            <Loader2 className="h-8 w-8 text-sky-600 dark:text-sky-400 animate-spin mx-auto" />
            <p className="text-sm text-slate-800 dark:text-slate-200 font-bold">Executando Descoberta Multicamada Inteligente...</p>
            <p className="text-xs text-slate-500 dark:text-slate-400 font-mono">Sondando ARP, SADP Hikvision, ONVIF e portas TCP</p>
          </div>
        ) : filteredDevices.length === 0 ? (
          <div className="py-16 text-center text-slate-500 text-xs">
            Nenhum dispositivo encontrado para os filtros selecionados.
          </div>
        ) : (
          <table className="w-full text-left text-xs font-mono border-collapse">
            <thead className="bg-slate-50 dark:bg-slate-900 text-slate-600 dark:text-slate-400 uppercase text-[10px] tracking-wider border-b border-slate-200 dark:border-slate-800 sticky top-0 backdrop-blur z-10 shadow-sm">
              <tr>
                <th className="px-3 py-2.5 w-8 text-center">
                  <input
                    type="checkbox"
                    checked={
                      unaddedInFilter.length > 0 &&
                      unaddedInFilter.every((d) => selectedIps.has(d.ip))
                    }
                    onChange={toggleSelectAllFiltered}
                    disabled={unaddedInFilter.length === 0}
                    className="rounded border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-950 text-sky-600 focus:ring-sky-500 h-3.5 w-3.5 cursor-pointer disabled:opacity-30"
                  />
                </th>
                <th className="px-2.5 py-2.5 w-10 text-center text-slate-400 dark:text-slate-500">Nº</th>
                <SortableHeader label="Endereço IP" column="ip" sortColumn={sortColumn} sortDirection={sortDirection} onSort={handleSort} />
                <SortableHeader label="Tipo" column="type" sortColumn={sortColumn} sortDirection={sortDirection} onSort={handleSort} />
                <SortableHeader label="Modelo" column="model" sortColumn={sortColumn} sortDirection={sortDirection} onSort={handleSort} />
                <SortableHeader label="Status" column="status" sortColumn={sortColumn} sortDirection={sortDirection} onSort={handleSort} />
                <th className="px-3 py-2.5">Nº Série / MAC</th>
                <th className="px-3 py-2.5">Portas / Protocolos</th>
                <SortableHeader label="Confiança" column="confidence" sortColumn={sortColumn} sortDirection={sortDirection} onSort={handleSort} />
                <th className="px-3 py-2.5 text-right">Ações</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 dark:divide-slate-800/60 bg-white dark:bg-slate-950">
              {filteredDevices.map((dev, idx) => {
                const isSelected = selectedIps.has(dev.ip);
                const isAdded = dev.is_already_added;
                const hasVideo = ['ip_camera', 'nvr', 'dvr', 'intercom', 'ptz', 'traffic_lpr', 'thermal'].includes(dev.device_type);
                const isEvidencesOpen = expandedEvidencesIp === dev.ip;

                return (
                  <React.Fragment key={dev.ip}>
                    <tr
                      className={`hover:bg-sky-50/60 dark:hover:bg-slate-900/60 transition ${
                        isSelected ? 'bg-sky-50 dark:bg-sky-500/10' : ''
                      }`}
                    >
                      {/* Checkbox */}
                      <td className="px-3 py-2 text-center">
                        <input
                          type="checkbox"
                          checked={isSelected}
                          disabled={isAdded}
                          onChange={() => toggleSelect(dev.ip)}
                          className="rounded border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-950 text-sky-600 focus:ring-sky-500 h-3.5 w-3.5 cursor-pointer disabled:opacity-30"
                        />
                      </td>

                      {/* Nº */}
                      <td className="px-2.5 py-2 text-center text-slate-400 dark:text-slate-500 font-mono text-[11px]">
                        {idx + 1}
                      </td>

                      {/* IP (Clickable Blue Link) */}
                      <td className="px-3 py-2">
                        <div className="flex items-center gap-1 font-bold text-sky-600 dark:text-sky-400 text-xs">
                          <span
                            onClick={() => hasVideo && setQuickViewDevice(dev)}
                            className={hasVideo ? 'cursor-pointer hover:underline' : ''}
                            title={hasVideo ? 'Abrir visualização ao vivo' : undefined}
                          >
                            {dev.ip}
                          </span>
                          <button
                            onClick={() => handleCopy(dev.ip)}
                            className="text-slate-400 hover:text-slate-600 dark:hover:text-slate-300 p-0.5"
                            title={copiedText === dev.ip ? 'Copiado!' : 'Copiar IP'}
                          >
                            {copiedText === dev.ip ? (
                              <Check className="h-3 w-3 text-emerald-500" />
                            ) : (
                              <Copy className="h-3 w-3" />
                            )}
                          </button>
                        </div>
                        <span className="text-[10px] text-slate-400 dark:text-slate-500 block">
                          Porta RTSP: {dev.rtsp_port || 554}
                        </span>
                      </td>

                      {/* Type Badge */}
                      <td className="px-3 py-2">
                        <span
                          className={`inline-flex items-center gap-1 px-2 py-0.5 rounded-md text-[10px] font-semibold border ${getTypeBadgeClass(
                            dev.device_type
                          )}`}
                        >
                          {getTypeIcon(dev.device_type)}
                          <span>{dev.device_type_label}</span>
                        </span>
                      </td>

                      {/* Brand & Model */}
                      <td className="px-3 py-2 font-sans">
                        <div className="font-bold text-slate-800 dark:text-slate-200 leading-tight">
                          {dev.hardware_model || dev.name}
                        </div>
                        <div className="text-[11px] text-sky-600 dark:text-sky-400 font-mono mt-0.5">
                          {dev.brand}
                        </div>
                      </td>

                      {/* Status / Activation */}
                      <td className="px-3 py-2">
                        <span
                          className={`text-[11px] font-bold ${
                            dev.activation_status === 'Aguardando ativação'
                              ? 'text-rose-600 dark:text-rose-400'
                              : 'text-emerald-600 dark:text-emerald-400'
                          }`}
                        >
                          {dev.activation_status === 'Ativado' ? 'ATIVADO' : dev.activation_status}
                        </span>
                      </td>

                      {/* Serial / MAC */}
                      <td className="px-3 py-2">
                        <div className="text-[11px] text-slate-700 dark:text-slate-300 font-mono">
                          {dev.serial_number ? dev.serial_number.slice(-10) : '—'}
                        </div>
                        <div className="text-[10px] text-slate-400 dark:text-slate-500 font-mono">
                          {dev.mac || 'MAC não resolvido'}
                        </div>
                      </td>

                      {/* Ports & Protocols */}
                      <td className="px-3 py-2">
                        <div className="flex flex-wrap gap-1 max-w-[140px]">
                          {(dev.protocols || []).map((p) => (
                            <span
                              key={p}
                              className="px-1 py-0.2 rounded text-[9px] font-bold bg-slate-100 dark:bg-slate-900 text-slate-700 dark:text-slate-300 border border-slate-200 dark:border-slate-800"
                            >
                              {p}
                            </span>
                          ))}
                        </div>
                      </td>

                      {/* Confidence Score & Evidence Button */}
                      <td className="px-3 py-2">
                        <div className="flex items-center gap-1.5">
                          <span
                            className={`px-1.5 py-0.2 rounded text-[9px] font-mono font-bold border ${
                              dev.confidence_score >= 90
                                ? 'bg-emerald-50 dark:bg-emerald-500/15 text-emerald-700 dark:text-emerald-300 border-emerald-200 dark:border-emerald-500/30'
                                : dev.confidence_score >= 70
                                ? 'bg-sky-50 dark:bg-sky-500/15 text-sky-700 dark:text-sky-300 border-sky-200 dark:border-sky-500/30'
                                : 'bg-slate-100 dark:bg-slate-800 text-slate-600 dark:text-slate-400 border-slate-200 dark:border-slate-700'
                            }`}
                          >
                            {dev.confidence_score}%
                          </span>
                          {dev.evidences && dev.evidences.length > 0 && (
                            <button
                              onClick={() => setExpandedEvidencesIp(isEvidencesOpen ? null : dev.ip)}
                              className="p-1 text-slate-400 hover:text-sky-600 dark:hover:text-sky-300 hover:bg-slate-100 dark:hover:bg-slate-800 rounded"
                              title="Ver evidências"
                            >
                              <Info className="h-3 w-3" />
                            </button>
                          )}
                        </div>
                      </td>

                      {/* Actions */}
                      <td className="px-3 py-2 text-right font-sans">
                        <div className="flex items-center justify-end gap-1.5">
                          {hasVideo && (
                            <button
                              onClick={() => setQuickViewDevice(dev)}
                              className="px-2.5 py-1 rounded bg-sky-50 dark:bg-sky-500/20 hover:bg-sky-100 dark:hover:bg-sky-500/35 text-sky-600 dark:text-sky-300 border border-sky-200 dark:border-sky-500/40 text-xs font-bold inline-flex items-center gap-1 transition shadow-sm"
                              title="Visualizar tela completa e configurar OSD"
                            >
                              <Eye className="h-3.5 w-3.5 text-sky-600 dark:text-sky-400" />
                              <span>Visualizar</span>
                            </button>
                          )}

                          {!isAdded ? (
                            <button
                              onClick={() => {
                                onAddSingle({
                                  name: dev.name,
                                  host: dev.ip,
                                  username: 'admin',
                                  rtsp_port: dev.rtsp_port || 554,
                                  http_port: dev.http_port || 80,
                                  stream_profile: 'main',
                                });
                              }}
                              className="px-2 py-1 rounded bg-slate-100 dark:bg-slate-800 hover:bg-slate-200 dark:hover:bg-slate-700 text-slate-700 dark:text-slate-200 border border-slate-200 dark:border-slate-700 text-xs font-semibold inline-flex items-center gap-1 transition shadow-sm"
                              title="Adicionar à dashboard"
                            >
                              <Plus className="h-3 w-3" />
                              <span>Adicionar</span>
                            </button>
                          ) : (
                            <span className="inline-flex items-center gap-1 text-[11px] text-emerald-600 dark:text-emerald-400 font-semibold px-1">
                              <CheckCircle2 className="h-3 w-3" />
                              Cadastrada
                            </span>
                          )}
                        </div>
                      </td>
                    </tr>

                    {/* Expandable Evidences */}
                    {isEvidencesOpen && (
                      <tr className="bg-slate-50 dark:bg-slate-950 border-b border-slate-200 dark:border-slate-800">
                        <td colSpan={10} className="px-6 py-2.5">
                          <div className="flex flex-wrap gap-1.5 text-[10px]">
                            {dev.evidences?.map((ev, i) => (
                              <span key={i} className="px-2 py-0.5 rounded bg-emerald-50 dark:bg-emerald-500/10 text-emerald-700 dark:text-emerald-300 border border-emerald-200 dark:border-emerald-500/30">
                                {ev}
                              </span>
                            ))}
                            {dev.contradictions?.map((c, i) => (
                              <span key={i} className="px-2 py-0.5 rounded bg-rose-50 dark:bg-rose-500/10 text-rose-700 dark:text-rose-300 border border-rose-200 dark:border-rose-500/30">
                                {c}
                              </span>
                            ))}
                          </div>
                        </td>
                      </tr>
                    )}
                  </React.Fragment>
                );
              })}
            </tbody>
          </table>
        )}
      </div>

      {/* 4. BOTTOM FLOATING BATCH BAR */}
      <div className="px-5 py-2.5 bg-slate-50 dark:bg-slate-900 border-t border-slate-200 dark:border-slate-800 flex items-center justify-between text-xs shrink-0 transition-colors">
        <div className="flex items-center gap-3 text-slate-700 dark:text-slate-300">
          <span className="font-bold">
            Selecionado: <strong className="text-sky-600 dark:text-sky-400 font-mono text-sm">{selectedIps.size}</strong>
          </span>
          <span className="text-slate-300 dark:text-slate-600">|</span>
          <span className="text-slate-500 dark:text-slate-400 text-[11px]">
            Marque os dispositivos desejados para adicionar todos de uma vez
          </span>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={() => setIsBatchOpen(true)}
            disabled={selectedIps.size === 0}
            className="px-5 py-2 rounded-lg bg-sky-600 hover:bg-sky-500 text-white text-xs font-bold shadow-md shadow-sky-500/20 flex items-center gap-2 transition disabled:opacity-40 disabled:pointer-events-none"
          >
            <Layers className="h-4 w-4" />
            <span>Adicionar à lista de dispositivos ({selectedIps.size})</span>
          </button>
        </div>
      </div>

      {/* Quick Viewer Modal */}
      <QuickViewerModal
        device={quickViewDevice}
        isOpen={quickViewDevice !== null}
        onClose={() => setQuickViewDevice(null)}
        onDeviceUpdated={() => onRefreshScan(selectedInterface || undefined)}
        onAddAsCamera={(prefill) => {
          setQuickViewDevice(null);
          onAddSingle(prefill);
        }}
      />

      {/* Batch Password Modal */}
      {isBatchOpen && (
        <div className="fixed inset-0 z-60 bg-black/60 dark:bg-black/80 backdrop-blur-sm flex items-center justify-center p-4">
          <div className="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-xl shadow-2xl w-full max-w-md overflow-hidden animate-in fade-in zoom-in-95 duration-150">
            <div className="px-6 py-4 border-b border-slate-200 dark:border-slate-800 flex items-center justify-between bg-slate-50 dark:bg-slate-950/60">
              <h4 className="text-base font-bold text-slate-800 dark:text-white flex items-center gap-2">
                <Layers className="h-4 w-4 text-sky-600 dark:text-sky-400" />
                Cadastrar {selectedIps.size} Câmera(s) em Lote
              </h4>
              <button
                onClick={() => setIsBatchOpen(false)}
                className="p-1 rounded-lg text-slate-400 hover:text-slate-700 dark:hover:text-white hover:bg-slate-100 dark:hover:bg-slate-800"
              >
                ✕
              </button>
            </div>

            <form onSubmit={handleConfirmBatch} className="p-6 space-y-4">
              <p className="text-xs text-slate-600 dark:text-slate-300">
                Digite a senha de instalação apenas uma vez. Ela será criptografada em AES-256-GCM e aplicada a todas as {selectedIps.size} câmeras selecionadas:
              </p>

              {batchError && (
                <div className="p-3 rounded-lg bg-rose-50 dark:bg-rose-500/15 border border-rose-200 dark:border-rose-500/30 text-rose-700 dark:text-rose-300 text-xs flex items-center gap-2">
                  <AlertTriangle className="h-4 w-4 shrink-0" />
                  <span>{batchError}</span>
                </div>
              )}

              <div>
                <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 uppercase tracking-wider mb-1.5">
                  Usuário Padrão
                </label>
                <input
                  type="text"
                  required
                  value={batchUsername}
                  onChange={(e) => setBatchUsername(e.target.value)}
                  placeholder="admin"
                  className="w-full bg-slate-50 dark:bg-slate-950 border border-slate-300 dark:border-slate-800 rounded-lg px-3.5 py-2 text-sm text-slate-900 dark:text-white focus:outline-none focus:border-sky-500"
                />
              </div>

              <div>
                <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 uppercase tracking-wider mb-1.5">
                  Senha das Câmeras (Digitada uma única vez)
                </label>
                <div className="relative">
                  <input
                    type={showPassword ? 'text' : 'password'}
                    value={batchPassword}
                    onChange={(e) => setBatchPassword(e.target.value)}
                    placeholder="Digite a senha para todas"
                    className="w-full bg-slate-50 dark:bg-slate-950 border border-slate-300 dark:border-slate-800 rounded-lg pl-3.5 pr-10 py-2 text-sm text-slate-900 dark:text-white focus:outline-none focus:border-sky-500 font-mono"
                  />
                  <button
                    type="button"
                    onClick={() => setShowPassword(!showPassword)}
                    className="absolute right-2.5 top-1/2 -translate-y-1/2 text-slate-400 hover:text-slate-600 dark:hover:text-slate-200"
                  >
                    {showPassword ? '👁' : '🔒'}
                  </button>
                </div>
              </div>

              <div>
                <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 uppercase tracking-wider mb-1.5">
                  Perfil de Stream Padrão
                </label>
                <div className="grid grid-cols-2 gap-2">
                  <button
                    type="button"
                    onClick={() => setBatchProfile('main')}
                    className={`py-2 px-3 rounded-lg text-xs font-medium border text-center transition ${
                      batchProfile === 'main'
                        ? 'bg-sky-50 dark:bg-sky-500/20 border-sky-400 dark:border-sky-500/50 text-sky-700 dark:text-sky-300 font-bold'
                        : 'bg-white dark:bg-slate-950 border-slate-200 dark:border-slate-800 text-slate-600 dark:text-slate-400 hover:border-slate-300'
                    }`}
                  >
                    Principal (101)
                  </button>
                  <button
                    type="button"
                    onClick={() => setBatchProfile('sub')}
                    className={`py-2 px-3 rounded-lg text-xs font-medium border text-center transition ${
                      batchProfile === 'sub'
                        ? 'bg-sky-50 dark:bg-sky-500/20 border-sky-400 dark:border-sky-500/50 text-sky-700 dark:text-sky-300 font-bold'
                        : 'bg-white dark:bg-slate-950 border-slate-200 dark:border-slate-800 text-slate-600 dark:text-slate-400 hover:border-slate-300'
                    }`}
                  >
                    Secundário (102)
                  </button>
                </div>
              </div>

              <div className="pt-3 border-t border-slate-200 dark:border-slate-800 flex items-center justify-end gap-3">
                <button
                  type="button"
                  onClick={() => setIsBatchOpen(false)}
                  className="px-4 py-2 rounded-lg bg-slate-100 dark:bg-slate-800 hover:bg-slate-200 dark:hover:bg-slate-700 text-slate-700 dark:text-slate-300 text-xs font-medium transition"
                >
                  Voltar
                </button>
                <button
                  type="submit"
                  disabled={isSavingBatch}
                  className="px-5 py-2 rounded-lg bg-sky-600 hover:bg-sky-500 text-white text-xs font-bold shadow-md shadow-sky-500/20 transition flex items-center gap-2 disabled:opacity-50"
                >
                  {isSavingBatch ? (
                    <Loader2 className="h-4 w-4 animate-spin" />
                  ) : (
                    <Check className="h-4 w-4" />
                  )}
                  <span>Salvar e Cadastrar Todas</span>
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
};
