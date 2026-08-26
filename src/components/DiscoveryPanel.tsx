import React, { useState, useEffect, useMemo } from 'react';
import {
  Search,
  RefreshCw,
  Plus,
  Check,
  CheckCircle2,
  AlertTriangle,
  Eye,
  EyeOff,
  Loader2,
  Camera,
  Layers,
  Radio,
  Server,
  PhoneCall,
  Car,
  Compass,
  Thermometer,
  Boxes,
  ChevronDown,
  ChevronUp,
  Network,
  ShieldCheck,
  Copy,
  Info,
} from 'lucide-react';
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
}

export const DiscoveryPanel: React.FC<DiscoveryPanelProps> = ({
  discoveredDevices,
  isScanning,
  onRefreshScan,
  onAdded,
  onAddSingle,
}) => {
  const [interfaces, setInterfaces] = useState<NetworkInterfaceInfo[]>([]);
  const [selectedInterface, setSelectedInterface] = useState<string>('');
  const [activeFilter, setActiveFilter] = useState<string>('all');
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedIps, setSelectedIps] = useState<Set<string>>(new Set());
  const [isCollapsed, setIsCollapsed] = useState(false);
  const [copiedText, setCopiedText] = useState<string | null>(null);
  const [expandedEvidencesIp, setExpandedEvidencesIp] = useState<string | null>(null);

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
    return (discoveredDevices || []).filter((d) => {
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
  }, [discoveredDevices, activeFilter, searchQuery]);

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
        return <PhoneCall className="h-3.5 w-3.5 text-purple-400" />;
      case 'traffic_lpr':
        return <Car className="h-3.5 w-3.5 text-amber-400" />;
      case 'nvr':
      case 'dvr':
        return <Server className="h-3.5 w-3.5 text-blue-400" />;
      case 'ptz':
        return <Compass className="h-3.5 w-3.5 text-cyan-400" />;
      case 'thermal':
        return <Thermometer className="h-3.5 w-3.5 text-rose-400" />;
      case 'server':
        return <Server className="h-3.5 w-3.5 text-orange-400" />;
      case 'switch':
        return <Network className="h-3.5 w-3.5 text-indigo-400" />;
      case 'router':
        return <Network className="h-3.5 w-3.5 text-cyan-400" />;
      case 'ip_camera':
        return <Camera className="h-3.5 w-3.5 text-emerald-400" />;
      default:
        return <Boxes className="h-3.5 w-3.5 text-slate-400" />;
    }
  };

  const getTypeBadgeClass = (type: DeviceType) => {
    switch (type) {
      case 'intercom':
        return 'bg-purple-500/15 text-purple-300 border-purple-500/30';
      case 'traffic_lpr':
        return 'bg-amber-500/15 text-amber-300 border-amber-500/30';
      case 'nvr':
      case 'dvr':
        return 'bg-blue-500/15 text-blue-300 border-blue-500/30';
      case 'ptz':
        return 'bg-cyan-500/15 text-cyan-300 border-cyan-500/30';
      case 'thermal':
        return 'bg-rose-500/15 text-rose-300 border-rose-500/30';
      case 'server':
        return 'bg-orange-500/15 text-orange-300 border-orange-500/30';
      case 'switch':
        return 'bg-indigo-500/15 text-indigo-300 border-indigo-500/30';
      case 'router':
        return 'bg-cyan-500/15 text-cyan-300 border-cyan-500/30';
      case 'ip_camera':
        return 'bg-emerald-500/15 text-emerald-300 border-emerald-500/30';
      default:
        return 'bg-slate-800 text-slate-400 border-slate-700';
    }
  };

  return (
    <div className="bg-slate-900/95 border border-slate-800 rounded-xl overflow-hidden shadow-2xl transition-all">
      {/* Panel Header with Network Interface Selector */}
      <div className="px-5 py-3 bg-slate-950/90 border-b border-slate-800 flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          <div className="h-8 w-8 rounded-lg bg-sky-500/20 border border-sky-500/30 flex items-center justify-center text-sky-400">
            <Radio className="h-4 w-4 animate-pulse" />
          </div>
          <div>
            <div className="flex items-center gap-2">
              <h3 className="text-sm font-bold text-white tracking-tight">
                Descoberta & Quick Viewer de Dispositivos
              </h3>
              <span className="px-2 py-0.5 rounded-full text-[10px] font-mono font-bold bg-sky-500/20 text-sky-300 border border-sky-500/30">
                {(discoveredDevices || []).length} encontrados
              </span>
            </div>
            <p className="text-[11px] text-slate-400">
              Visualização ao vivo com 1 clique, alteração de Device Name e OSD diretamente na rede
            </p>
          </div>
        </div>

        {/* Network Interface Picker & Actions */}
        <div className="flex items-center gap-2.5">
          {/* NIC Selector */}
          <div className="flex items-center gap-1.5 bg-slate-900 border border-slate-800 rounded-lg px-2.5 py-1 text-xs">
            <Network className="h-3.5 w-3.5 text-sky-400 shrink-0" />
            <select
              value={selectedInterface}
              onChange={(e) => setSelectedInterface(e.target.value)}
              className="bg-transparent text-slate-200 text-xs font-semibold focus:outline-none cursor-pointer pr-1"
            >
              {(interfaces || []).map((iface) => (
                <option key={iface.id} value={iface.id} className="bg-slate-900 text-white">
                  {iface.name} - {iface.ip}/{iface.netmask}
                </option>
              ))}
            </select>
          </div>

          {/* Quick Search */}
          <div className="relative w-44 sm:w-56">
            <Search className="h-3.5 w-3.5 absolute left-3 top-1/2 -translate-y-1/2 text-slate-400" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="IP, MAC, modelo, tipo..."
              className="w-full bg-slate-900 border border-slate-800 rounded-lg pl-8 pr-3 py-1.5 text-xs text-white placeholder-slate-500 focus:outline-none focus:border-sky-500 font-sans"
            />
          </div>

          {/* Trigger Scan Button */}
          <button
            onClick={() => onRefreshScan(selectedInterface || undefined)}
            disabled={isScanning}
            className="px-3.5 py-1.5 rounded-lg bg-sky-600 hover:bg-sky-500 text-white text-xs font-bold flex items-center gap-1.5 transition disabled:opacity-50 shadow-md shadow-sky-950"
            title="Escanear a interface selecionada"
          >
            <RefreshCw className={`h-3.5 w-3.5 ${isScanning ? 'animate-spin' : ''}`} />
            <span>{isScanning ? 'Varrendo...' : 'Procurar Dispositivos'}</span>
          </button>

          {/* Collapse Button */}
          <button
            onClick={() => setIsCollapsed(!isCollapsed)}
            className="p-1.5 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800"
          >
            {isCollapsed ? <ChevronDown className="h-4 w-4" /> : <ChevronUp className="h-4 w-4" />}
          </button>
        </div>
      </div>

      {!isCollapsed && (
        <>
          {/* Active Network & Protocols Status Strip */}
          <div className="px-5 py-2 bg-slate-950/60 border-b border-slate-800/80 flex flex-wrap items-center justify-between gap-2 text-[11px] font-mono">
            <div className="flex items-center gap-3 text-slate-400">
              {currentIfaceInfo && (
                <>
                  <span>
                    IP Local: <strong className="text-slate-200">{currentIfaceInfo.ip}</strong>
                  </span>
                  <span>•</span>
                  <span>
                    Máscara: <strong className="text-slate-200">{currentIfaceInfo.netmask}</strong>
                  </span>
                  {currentIfaceInfo.gateway && (
                    <>
                      <span>•</span>
                      <span>
                        Gateway: <strong className="text-slate-200">{currentIfaceInfo.gateway}</strong>
                      </span>
                    </>
                  )}
                </>
              )}
            </div>

            {/* Protocol Badges */}
            <div className="flex items-center gap-1.5">
              <span className="px-1.5 py-0.5 rounded text-[10px] font-bold bg-emerald-500/10 text-emerald-300 border border-emerald-500/30">
                ✓ SADP:37020
              </span>
              <span className="px-1.5 py-0.5 rounded text-[10px] font-bold bg-emerald-500/10 text-emerald-300 border border-emerald-500/30">
                ✓ ISAPI / Digest
              </span>
              <span className="px-1.5 py-0.5 rounded text-[10px] font-bold bg-emerald-500/10 text-emerald-300 border border-emerald-500/30">
                ✓ ONVIF:3702
              </span>
              <span className="px-1.5 py-0.5 rounded text-[10px] font-bold bg-emerald-500/10 text-emerald-300 border border-emerald-500/30">
                ✓ RTSP:554
              </span>
            </div>
          </div>

          {/* Filter Pills / Tabs */}
          <div className="px-5 py-2.5 bg-slate-900 border-b border-slate-800/80 flex items-center gap-2 overflow-x-auto scrollbar-none text-xs">
            <button
              onClick={() => setActiveFilter('all')}
              className={`px-3 py-1 rounded-lg font-medium transition flex items-center gap-1.5 shrink-0 ${
                activeFilter === 'all'
                  ? 'bg-sky-600 text-white font-bold shadow'
                  : 'bg-slate-950/60 text-slate-400 hover:text-slate-200 hover:bg-slate-800 border border-slate-800/80'
              }`}
            >
              <Boxes className="h-3.5 w-3.5" />
              <span>Todos</span>
              <span className="px-1.5 py-0.2 rounded-full text-[10px] bg-black/40 font-mono">
                {counts.all}
              </span>
            </button>

            {counts.ip_camera > 0 && (
              <button
                onClick={() => setActiveFilter('ip_camera')}
                className={`px-3 py-1 rounded-lg font-medium transition flex items-center gap-1.5 shrink-0 ${
                  activeFilter === 'ip_camera'
                    ? 'bg-emerald-600 text-white font-bold shadow'
                    : 'bg-slate-950/60 text-slate-400 hover:text-slate-200 hover:bg-slate-800 border border-slate-800/80'
                }`}
              >
                <Camera className="h-3.5 w-3.5 text-emerald-400" />
                <span>Câmeras IP</span>
                <span className="px-1.5 py-0.2 rounded-full text-[10px] bg-black/40 font-mono">
                  {counts.ip_camera}
                </span>
              </button>
            )}

            {counts.intercom > 0 && (
              <button
                onClick={() => setActiveFilter('intercom')}
                className={`px-3 py-1 rounded-lg font-medium transition flex items-center gap-1.5 shrink-0 ${
                  activeFilter === 'intercom'
                    ? 'bg-purple-600 text-white font-bold shadow'
                    : 'bg-slate-950/60 text-slate-400 hover:text-slate-200 hover:bg-slate-800 border border-slate-800/80'
                }`}
              >
                <PhoneCall className="h-3.5 w-3.5 text-purple-400" />
                <span>Videoporteiros / Intercom</span>
                <span className="px-1.5 py-0.2 rounded-full text-[10px] bg-black/40 font-mono">
                  {counts.intercom}
                </span>
              </button>
            )}

            {counts.nvr > 0 && (
              <button
                onClick={() => setActiveFilter('nvr')}
                className={`px-3 py-1 rounded-lg font-medium transition flex items-center gap-1.5 shrink-0 ${
                  activeFilter === 'nvr'
                    ? 'bg-blue-600 text-white font-bold shadow'
                    : 'bg-slate-950/60 text-slate-400 hover:text-slate-200 hover:bg-slate-800 border border-slate-800/80'
                }`}
              >
                <Server className="h-3.5 w-3.5 text-blue-400" />
                <span>NVRs / Gravadores</span>
                <span className="px-1.5 py-0.2 rounded-full text-[10px] bg-black/40 font-mono">
                  {counts.nvr}
                </span>
              </button>
            )}

            {counts.server > 0 && (
              <button
                onClick={() => setActiveFilter('server')}
                className={`px-3 py-1 rounded-lg font-medium transition flex items-center gap-1.5 shrink-0 ${
                  activeFilter === 'server'
                    ? 'bg-orange-600 text-white font-bold shadow'
                    : 'bg-slate-950/60 text-slate-400 hover:text-slate-200 hover:bg-slate-800 border border-slate-800/80'
                }`}
              >
                <Server className="h-3.5 w-3.5 text-orange-400" />
                <span>Servidores</span>
                <span className="px-1.5 py-0.2 rounded-full text-[10px] bg-black/40 font-mono">
                  {counts.server}
                </span>
              </button>
            )}

            {counts.switch > 0 && (
              <button
                onClick={() => setActiveFilter('switch')}
                className={`px-3 py-1 rounded-lg font-medium transition flex items-center gap-1.5 shrink-0 ${
                  activeFilter === 'switch'
                    ? 'bg-indigo-600 text-white font-bold shadow'
                    : 'bg-slate-950/60 text-slate-400 hover:text-slate-200 hover:bg-slate-800 border border-slate-800/80'
                }`}
              >
                <Network className="h-3.5 w-3.5 text-indigo-400" />
                <span>Switches</span>
                <span className="px-1.5 py-0.2 rounded-full text-[10px] bg-black/40 font-mono">
                  {counts.switch}
                </span>
              </button>
            )}

            {counts.router > 0 && (
              <button
                onClick={() => setActiveFilter('router')}
                className={`px-3 py-1 rounded-lg font-medium transition flex items-center gap-1.5 shrink-0 ${
                  activeFilter === 'router'
                    ? 'bg-cyan-600 text-white font-bold shadow'
                    : 'bg-slate-950/60 text-slate-400 hover:text-slate-200 hover:bg-slate-800 border border-slate-800/80'
                }`}
              >
                <Network className="h-3.5 w-3.5 text-cyan-400" />
                <span>Roteadores</span>
                <span className="px-1.5 py-0.2 rounded-full text-[10px] bg-black/40 font-mono">
                  {counts.router}
                </span>
              </button>
            )}

            {counts.traffic_lpr > 0 && (
              <button
                onClick={() => setActiveFilter('traffic_lpr')}
                className={`px-3 py-1 rounded-lg font-medium transition flex items-center gap-1.5 shrink-0 ${
                  activeFilter === 'traffic_lpr'
                    ? 'bg-amber-600 text-white font-bold shadow'
                    : 'bg-slate-950/60 text-slate-400 hover:text-slate-200 hover:bg-slate-800 border border-slate-800/80'
                }`}
              >
                <Car className="h-3.5 w-3.5 text-amber-400" />
                <span>Tráfego / LPR</span>
                <span className="px-1.5 py-0.2 rounded-full text-[10px] bg-black/40 font-mono">
                  {counts.traffic_lpr}
                </span>
              </button>
            )}

            {counts.ptz > 0 && (
              <button
                onClick={() => setActiveFilter('ptz')}
                className={`px-3 py-1 rounded-lg font-medium transition flex items-center gap-1.5 shrink-0 ${
                  activeFilter === 'ptz'
                    ? 'bg-cyan-600 text-white font-bold shadow'
                    : 'bg-slate-950/60 text-slate-400 hover:text-slate-200 hover:bg-slate-800 border border-slate-800/80'
                }`}
              >
                <Compass className="h-3.5 w-3.5 text-cyan-400" />
                <span>PTZ / Speed Dome</span>
                <span className="px-1.5 py-0.2 rounded-full text-[10px] bg-black/40 font-mono">
                  {counts.ptz}
                </span>
              </button>
            )}

            {counts.other > 0 && (
              <button
                onClick={() => setActiveFilter('other')}
                className={`px-3 py-1 rounded-lg font-medium transition flex items-center gap-1.5 shrink-0 ${
                  activeFilter === 'other'
                    ? 'bg-slate-700 text-white font-bold shadow'
                    : 'bg-slate-950/60 text-slate-400 hover:text-slate-200 hover:bg-slate-800 border border-slate-800/80'
                }`}
              >
                <Boxes className="h-3.5 w-3.5 text-slate-400" />
                <span>Não Identificados</span>
                <span className="px-1.5 py-0.2 rounded-full text-[10px] bg-black/40 font-mono">
                  {counts.other}
                </span>
              </button>
            )}

            {counts.with_issues > 0 && (
              <button
                onClick={() => setActiveFilter('with_issues')}
                className={`px-3 py-1 rounded-lg font-medium transition flex items-center gap-1.5 shrink-0 ${
                  activeFilter === 'with_issues'
                    ? 'bg-amber-600 text-white font-bold shadow'
                    : 'bg-slate-950/60 text-amber-400 hover:text-amber-200 hover:bg-slate-800 border border-amber-500/30'
                }`}
              >
                <AlertTriangle className="h-3.5 w-3.5 text-amber-400" />
                <span>Com Alertas</span>
                <span className="px-1.5 py-0.2 rounded-full text-[10px] bg-black/40 font-mono">
                  {counts.with_issues}
                </span>
              </button>
            )}
          </div>

          {/* Professional Table Area */}
          <div className="max-h-72 overflow-y-auto">
            {isScanning && (discoveredDevices || []).length === 0 ? (
              <div className="py-12 text-center space-y-2">
                <Loader2 className="h-7 w-7 text-sky-400 animate-spin mx-auto" />
                <p className="text-xs text-slate-200 font-semibold">Executando Descoberta Multicamada Inteligente...</p>
                <p className="text-[11px] text-slate-400 font-mono">Consultando ARP, SADP, ONVIF e varredura TCP na sub-rede</p>
              </div>
            ) : filteredDevices.length === 0 ? (
              <div className="py-8 text-center text-slate-500 text-xs">
                Nenhum dispositivo encontrado neste filtro.
              </div>
            ) : (
              <table className="w-full text-left text-xs font-mono">
                <thead className="bg-slate-950/60 text-slate-400 uppercase text-[10px] tracking-wider border-b border-slate-800 sticky top-0 backdrop-blur">
                  <tr>
                    <th className="px-3.5 py-2.5 w-8">
                      <input
                        type="checkbox"
                        checked={
                          unaddedInFilter.length > 0 &&
                          unaddedInFilter.every((d) => selectedIps.has(d.ip))
                        }
                        onChange={toggleSelectAllFiltered}
                        disabled={unaddedInFilter.length === 0}
                        className="rounded border-slate-700 bg-slate-900 text-sky-500 focus:ring-sky-500 h-3.5 w-3.5 cursor-pointer disabled:opacity-30"
                      />
                    </th>
                    <th className="px-3.5 py-2.5">Status & Tipo</th>
                    <th className="px-3.5 py-2.5">Fabricante / Modelo</th>
                    <th className="px-3.5 py-2.5">IP & MAC</th>
                    <th className="px-3.5 py-2.5">Portas & Protocolos</th>
                    <th className="px-3.5 py-2.5">Ativação / Confiança</th>
                    <th className="px-3.5 py-2.5">Diagnóstico & Evidências</th>
                    <th className="px-3.5 py-2.5 text-right">Ações</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-slate-800/50">
                  {filteredDevices.map((dev) => {
                    const isSelected = selectedIps.has(dev.ip);
                    const isAdded = dev.is_already_added;
                    const hasIssues = dev.issues && dev.issues.length > 0;
                    const isEvidencesOpen = expandedEvidencesIp === dev.ip;
                    const hasVideo = ['ip_camera', 'nvr', 'dvr', 'intercom', 'ptz', 'traffic_lpr', 'thermal'].includes(dev.device_type);

                    return (
                      <React.Fragment key={dev.ip}>
                        <tr
                          className={`hover:bg-slate-800/40 transition ${
                            isSelected ? 'bg-sky-500/5' : ''
                          }`}
                        >
                          {/* Checkbox */}
                          <td className="px-3.5 py-2.5">
                            <input
                              type="checkbox"
                              checked={isSelected}
                              disabled={isAdded}
                              onChange={() => toggleSelect(dev.ip)}
                              className="rounded border-slate-700 bg-slate-900 text-sky-500 focus:ring-sky-500 h-3.5 w-3.5 cursor-pointer disabled:opacity-30"
                            />
                          </td>

                          {/* Status & Type */}
                          <td className="px-3.5 py-2.5">
                            <div className="flex items-center gap-1.5">
                              <span
                                className={`inline-flex items-center gap-1 px-2 py-0.5 rounded-md text-[10px] font-semibold border ${getTypeBadgeClass(
                                  dev.device_type
                                )}`}
                              >
                                {getTypeIcon(dev.device_type)}
                                <span>{dev.device_type_label}</span>
                              </span>
                            </div>
                          </td>

                          {/* Brand & Model */}
                          <td className="px-3.5 py-2.5 font-sans">
                            <div className="font-bold text-white leading-tight">{dev.hardware_model}</div>
                            <div className="text-[11px] text-sky-400 font-mono mt-0.5 flex items-center gap-1.5">
                              <span>{dev.brand}</span>
                              {dev.serial_number && (
                                <span className="text-slate-400 text-[10px]">
                                  • SN: {dev.serial_number.slice(-8)}
                                </span>
                              )}
                            </div>
                          </td>

                          {/* IP & MAC */}
                          <td className="px-3.5 py-2.5">
                            <div className="flex items-center gap-1 text-slate-200 font-bold">
                              <span>{dev.ip}</span>
                              <button
                                onClick={() => handleCopy(dev.ip)}
                                className="text-slate-500 hover:text-slate-300 p-0.5"
                                title={copiedText === dev.ip ? 'Copiado!' : 'Copiar IP'}
                              >
                                {copiedText === dev.ip ? (
                                  <Check className="h-3 w-3 text-emerald-400" />
                                ) : (
                                  <Copy className="h-3 w-3" />
                                )}
                              </button>
                            </div>
                            {dev.mac ? (
                              <div className="text-[10px] text-slate-400 mt-0.5">{dev.mac}</div>
                            ) : (
                              <div className="text-[10px] text-slate-600 italic">MAC indisponível</div>
                            )}
                          </td>

                          {/* Ports & Protocols */}
                          <td className="px-3.5 py-2.5">
                            <div className="flex flex-wrap gap-1">
                              {(dev.protocols || []).map((p) => (
                                <span
                                  key={p}
                                  className="px-1.5 py-0.2 rounded text-[9px] font-bold bg-slate-800 text-slate-300 border border-slate-700"
                                >
                                  {p}
                                </span>
                              ))}
                            </div>
                          </td>

                          {/* Activation & Confidence */}
                          <td className="px-3.5 py-2.5">
                            <div className="flex items-center gap-2">
                              <span
                                className={`text-[10px] font-bold ${
                                  dev.activation_status === 'Aguardando ativação'
                                    ? 'text-rose-400'
                                    : 'text-emerald-400'
                                }`}
                              >
                                {dev.activation_status || 'Ativo'}
                              </span>
                              <span
                                className={`px-1.5 py-0.2 rounded text-[9px] font-mono font-bold border ${
                                  dev.confidence_score >= 90
                                    ? 'bg-emerald-500/15 text-emerald-300 border-emerald-500/30'
                                    : dev.confidence_score >= 70
                                    ? 'bg-sky-500/15 text-sky-300 border-sky-500/30'
                                    : dev.confidence_score >= 40
                                    ? 'bg-amber-500/15 text-amber-300 border-amber-500/30'
                                    : 'bg-slate-800 text-slate-400 border-slate-700'
                                }`}
                                title={dev.confidence_level}
                              >
                                {dev.confidence_score}%
                              </span>
                            </div>
                          </td>

                          {/* Diagnostics & Evidences Button */}
                          <td className="px-3.5 py-2.5">
                            <div className="flex items-center justify-between gap-1">
                              {hasIssues ? (
                                <div className="space-y-0.5">
                                  {(dev.issues || []).map((issue, idx) => (
                                    <div
                                      key={idx}
                                      className="text-[10px] text-amber-300 flex items-center gap-1 leading-tight"
                                    >
                                      <span>{issue}</span>
                                    </div>
                                  ))}
                                </div>
                              ) : (
                                <span className="inline-flex items-center gap-1 text-[10px] text-emerald-400 font-semibold">
                                  <ShieldCheck className="h-3 w-3" />
                                  Sem anomalias
                                </span>
                              )}

                              {dev.evidences && dev.evidences.length > 0 && (
                                <button
                                  onClick={() => setExpandedEvidencesIp(isEvidencesOpen ? null : dev.ip)}
                                  className="p-1 rounded text-slate-400 hover:text-sky-300 hover:bg-slate-800 shrink-0"
                                  title="Ver evidências da classificação"
                                >
                                  <Info className="h-3.5 w-3.5" />
                                </button>
                              )}
                            </div>
                          </td>

                          {/* Actions: Quick View & Add */}
                          <td className="px-3.5 py-2.5 text-right font-sans">
                            <div className="flex items-center justify-end gap-1.5">
                              {/* Quick View Button for Video Devices */}
                              {hasVideo && (
                                <button
                                  onClick={() => setQuickViewDevice(dev)}
                                  className="px-2.5 py-1 rounded-lg bg-sky-500/20 hover:bg-sky-500/35 text-sky-300 border border-sky-500/40 text-xs font-bold inline-flex items-center gap-1 transition shadow"
                                  title="Visualizar imagem ao vivo e configurar OSD/Device Name"
                                >
                                  <Eye className="h-3.5 w-3.5 text-sky-400" />
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
                                      stream_profile: 'main',
                                    });
                                  }}
                                  className="px-2.5 py-1 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-200 border border-slate-700 text-xs font-semibold inline-flex items-center gap-1 transition shadow"
                                  title="Cadastrar na dashboard"
                                >
                                  <Plus className="h-3 w-3" />
                                  <span>Adicionar</span>
                                </button>
                              ) : (
                                <span className="inline-flex items-center gap-1 text-[11px] text-emerald-400 font-semibold px-1">
                                  <CheckCircle2 className="h-3 w-3" />
                                  Cadastrada
                                </span>
                              )}
                            </div>
                          </td>
                        </tr>

                        {/* Expandable Evidences Row */}
                        {isEvidencesOpen && (
                          <tr className="bg-slate-950/80 border-b border-slate-800/80">
                            <td colSpan={8} className="px-6 py-3">
                              <div className="space-y-2">
                                <div className="text-[11px] font-bold text-sky-400 flex items-center gap-1.5">
                                  <Info className="h-3.5 w-3.5" />
                                  <span>Evidências Coletadas para Classificação:</span>
                                </div>
                                <div className="flex flex-wrap gap-1.5">
                                  {dev.evidences?.map((ev, idx) => (
                                    <span
                                      key={idx}
                                      className="px-2 py-0.5 rounded text-[10px] font-mono bg-emerald-500/10 text-emerald-300 border border-emerald-500/30"
                                    >
                                      {ev}
                                    </span>
                                  ))}
                                  {dev.contradictions?.map((contra, idx) => (
                                    <span
                                      key={idx}
                                      className="px-2 py-0.5 rounded text-[10px] font-mono bg-rose-500/10 text-rose-300 border border-rose-500/30"
                                    >
                                      {contra}
                                    </span>
                                  ))}
                                </div>
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

          {/* Bottom Batch Action Toolbar */}
          <div className="px-5 py-2.5 bg-slate-950 border-t border-slate-800 flex items-center justify-between text-xs">
            <div className="text-slate-400 flex items-center gap-2">
              {selectedIps.size > 0 ? (
                <span className="text-sky-400 font-semibold flex items-center gap-1.5">
                  <Check className="h-3.5 w-3.5" />
                  {selectedIps.size} dispositivo(s) selecionado(s) para cadastro
                </span>
              ) : (
                <span>Marque as caixas para cadastrar múltiplos equipamentos digitando a senha apenas uma vez</span>
              )}
            </div>

            <button
              onClick={() => setIsBatchOpen(true)}
              disabled={selectedIps.size === 0}
              className="px-4 py-1.5 rounded-lg bg-sky-600 hover:bg-sky-500 text-white text-xs font-bold shadow-md shadow-sky-950 flex items-center gap-1.5 transition disabled:opacity-50 disabled:pointer-events-none"
            >
              <Layers className="h-3.5 w-3.5" />
              <span>Adicionar Selecionadas em Lote ({selectedIps.size})</span>
            </button>
          </div>
        </>
      )}

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

      {/* Nested Batch Password Modal */}
      {isBatchOpen && (
        <div className="fixed inset-0 z-60 bg-black/80 backdrop-blur-md flex items-center justify-center p-4">
          <div className="bg-slate-900 border border-slate-800 rounded-xl shadow-2xl w-full max-w-md overflow-hidden animate-in fade-in zoom-in-95 duration-150">
            <div className="px-6 py-4 border-b border-slate-800 flex items-center justify-between bg-slate-950/60">
              <h4 className="text-base font-bold text-white flex items-center gap-2">
                <Layers className="h-4 w-4 text-sky-400" />
                Cadastrar {selectedIps.size} Câmera(s) em Lote
              </h4>
              <button
                onClick={() => setIsBatchOpen(false)}
                className="p-1 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800"
              >
                ✕
              </button>
            </div>

            <form onSubmit={handleConfirmBatch} className="p-6 space-y-4">
              <p className="text-xs text-slate-300">
                Digite a senha de instalação apenas uma vez. Ela será criptografada em AES-256-GCM e aplicada a todas as {selectedIps.size} câmeras selecionadas:
              </p>

              {batchError && (
                <div className="p-3 rounded-lg bg-rose-500/15 border border-rose-500/30 text-rose-300 text-xs flex items-center gap-2">
                  <AlertTriangle className="h-4 w-4 shrink-0" />
                  <span>{batchError}</span>
                </div>
              )}

              <div>
                <label className="block text-xs font-semibold text-slate-300 uppercase tracking-wider mb-1.5">
                  Usuário Padrão
                </label>
                <input
                  type="text"
                  required
                  value={batchUsername}
                  onChange={(e) => setBatchUsername(e.target.value)}
                  placeholder="admin"
                  className="w-full bg-slate-950 border border-slate-800 rounded-lg px-3.5 py-2 text-sm text-white focus:outline-none focus:border-sky-500"
                />
              </div>

              <div>
                <label className="block text-xs font-semibold text-slate-300 uppercase tracking-wider mb-1.5">
                  Senha das Câmeras (Digitada uma única vez)
                </label>
                <div className="relative">
                  <input
                    type={showPassword ? 'text' : 'password'}
                    value={batchPassword}
                    onChange={(e) => setBatchPassword(e.target.value)}
                    placeholder="Digite a senha para todas"
                    className="w-full bg-slate-950 border border-slate-800 rounded-lg pl-3.5 pr-10 py-2 text-sm text-white focus:outline-none focus:border-sky-500"
                  />
                  <button
                    type="button"
                    onClick={() => setShowPassword(!showPassword)}
                    className="absolute right-2.5 top-1/2 -translate-y-1/2 text-slate-400 hover:text-slate-200"
                  >
                    {showPassword ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                  </button>
                </div>
              </div>

              <div>
                <label className="block text-xs font-semibold text-slate-300 uppercase tracking-wider mb-1.5">
                  Perfil de Stream Padrão
                </label>
                <div className="grid grid-cols-2 gap-2">
                  <button
                    type="button"
                    onClick={() => setBatchProfile('main')}
                    className={`py-2 px-3 rounded-lg text-xs font-medium border text-center transition ${
                      batchProfile === 'main'
                        ? 'bg-sky-500/20 border-sky-500/50 text-sky-300'
                        : 'bg-slate-950 border-slate-800 text-slate-400 hover:border-slate-700'
                    }`}
                  >
                    Principal (101)
                  </button>
                  <button
                    type="button"
                    onClick={() => setBatchProfile('sub')}
                    className={`py-2 px-3 rounded-lg text-xs font-medium border text-center transition ${
                      batchProfile === 'sub'
                        ? 'bg-sky-500/20 border-sky-500/50 text-sky-300'
                        : 'bg-slate-950 border-slate-800 text-slate-400 hover:border-slate-700'
                    }`}
                  >
                    Secundário (102)
                  </button>
                </div>
              </div>

              <div className="pt-3 border-t border-slate-800 flex items-center justify-end gap-3">
                <button
                  type="button"
                  onClick={() => setIsBatchOpen(false)}
                  className="px-4 py-2 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-300 text-xs font-medium transition"
                >
                  Voltar
                </button>
                <button
                  type="submit"
                  disabled={isSavingBatch}
                  className="px-5 py-2 rounded-lg bg-sky-600 hover:bg-sky-500 text-white text-xs font-bold shadow-md shadow-sky-950 transition flex items-center gap-2 disabled:opacity-50"
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
