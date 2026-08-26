import React, { useState, useMemo } from 'react';
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
} from 'lucide-react';
import type {
  DiscoveredDevice,
  DeviceType,
  BatchCreateCamerasInput,
  CreateCameraInput,
} from '@/types';
import { api } from '@/services/api';

interface DiscoveryPanelProps {
  discoveredDevices: DiscoveredDevice[];
  isScanning: boolean;
  onRefreshScan: () => void;
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
  const [activeFilter, setActiveFilter] = useState<string>('all');
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedIps, setSelectedIps] = useState<Set<string>>(new Set());
  const [isCollapsed, setIsCollapsed] = useState(false);

  // Batch modal state
  const [isBatchOpen, setIsBatchOpen] = useState(false);
  const [batchUsername, setBatchUsername] = useState('admin');
  const [batchPassword, setBatchPassword] = useState('');
  const [showPassword, setShowPassword] = useState(false);
  const [batchProfile, setBatchProfile] = useState<'main' | 'sub'>('main');
  const [isSavingBatch, setIsSavingBatch] = useState(false);
  const [batchError, setBatchError] = useState<string | null>(null);

  // Filter Counts
  const counts = useMemo(() => {
    const map: Record<string, number> = {
      all: discoveredDevices.length,
      ip_camera: 0,
      intercom: 0,
      nvr: 0,
      traffic_lpr: 0,
      ptz: 0,
      thermal: 0,
      other: 0,
    };
    for (const d of discoveredDevices) {
      if (map[d.device_type] !== undefined) {
        map[d.device_type]++;
      } else {
        map.other++;
      }
    }
    return map;
  }, [discoveredDevices]);

  // Filtered devices list
  const filteredDevices = useMemo(() => {
    return discoveredDevices.filter((d) => {
      const matchesType = activeFilter === 'all' || d.device_type === activeFilter;
      const q = searchQuery.toLowerCase().trim();
      const matchesQuery =
        !q ||
        d.name.toLowerCase().includes(q) ||
        d.ip.toLowerCase().includes(q) ||
        d.hardware_model.toLowerCase().includes(q) ||
        d.brand.toLowerCase().includes(q) ||
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

  const handleConfirmBatch = async (e: React.FormEvent) => {
    e.preventDefault();
    if (selectedIps.size === 0) return;

    setIsSavingBatch(true);
    setBatchError(null);

    try {
      const selectedDevices = discoveredDevices.filter((d) => selectedIps.has(d.ip));
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
        return <Server className="h-3.5 w-3.5 text-blue-400" />;
      case 'ptz':
        return <Compass className="h-3.5 w-3.5 text-cyan-400" />;
      case 'thermal':
        return <Thermometer className="h-3.5 w-3.5 text-rose-400" />;
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
        return 'bg-blue-500/15 text-blue-300 border-blue-500/30';
      case 'ptz':
        return 'bg-cyan-500/15 text-cyan-300 border-cyan-500/30';
      case 'thermal':
        return 'bg-rose-500/15 text-rose-300 border-rose-500/30';
      case 'ip_camera':
        return 'bg-emerald-500/15 text-emerald-300 border-emerald-500/30';
      default:
        return 'bg-slate-800 text-slate-300 border-slate-700';
    }
  };

  return (
    <div className="bg-slate-900/95 border border-slate-800 rounded-xl overflow-hidden shadow-2xl transition-all">
      {/* Panel Header */}
      <div className="px-5 py-3.5 bg-slate-950/80 border-b border-slate-800 flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="h-8 w-8 rounded-lg bg-sky-500/20 border border-sky-500/30 flex items-center justify-center text-sky-400">
            <Radio className="h-4 w-4 animate-pulse" />
          </div>
          <div>
            <div className="flex items-center gap-2">
              <h3 className="text-sm font-bold text-white tracking-tight">
                Busca Inteligente de Dispositivos na Rede
              </h3>
              <span className="px-2 py-0.5 rounded-full text-[10px] font-mono font-bold bg-sky-500/20 text-sky-300 border border-sky-500/30">
                {discoveredDevices.length} encontrados
              </span>
            </div>
            <p className="text-[11px] text-slate-400">
              Detecção e classificação automática de equipamentos ONVIF / Hikvision
            </p>
          </div>
        </div>

        <div className="flex items-center gap-2.5">
          {/* Quick Search Input */}
          <div className="relative w-48 sm:w-64">
            <Search className="h-3.5 w-3.5 absolute left-3 top-1/2 -translate-y-1/2 text-slate-400" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Filtrar por IP, modelo..."
              className="w-full bg-slate-900 border border-slate-800 rounded-lg pl-8 pr-3 py-1.5 text-xs text-white placeholder-slate-500 focus:outline-none focus:border-sky-500 font-sans"
            />
          </div>

          {/* Refresh Button */}
          <button
            onClick={onRefreshScan}
            disabled={isScanning}
            className="px-3 py-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-200 border border-slate-700 text-xs font-semibold flex items-center gap-1.5 transition disabled:opacity-50"
            title="Escanear rede local novamente"
          >
            <RefreshCw className={`h-3.5 w-3.5 ${isScanning ? 'animate-spin text-sky-400' : ''}`} />
            <span className="hidden sm:inline">{isScanning ? 'Buscando...' : 'Buscar'}</span>
          </button>

          {/* Collapse Toggle */}
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

            {counts.thermal > 0 && (
              <button
                onClick={() => setActiveFilter('thermal')}
                className={`px-3 py-1 rounded-lg font-medium transition flex items-center gap-1.5 shrink-0 ${
                  activeFilter === 'thermal'
                    ? 'bg-rose-600 text-white font-bold shadow'
                    : 'bg-slate-950/60 text-slate-400 hover:text-slate-200 hover:bg-slate-800 border border-slate-800/80'
                }`}
              >
                <Thermometer className="h-3.5 w-3.5 text-rose-400" />
                <span>Térmicas</span>
                <span className="px-1.5 py-0.2 rounded-full text-[10px] bg-black/40 font-mono">
                  {counts.thermal}
                </span>
              </button>
            )}
          </div>

          {/* Table Area */}
          <div className="max-h-60 overflow-y-auto">
            {isScanning && discoveredDevices.length === 0 ? (
              <div className="py-8 text-center space-y-2">
                <Loader2 className="h-6 w-6 text-sky-400 animate-spin mx-auto" />
                <p className="text-xs text-slate-300 font-medium">Varrendo a sub-rede local...</p>
              </div>
            ) : filteredDevices.length === 0 ? (
              <div className="py-8 text-center text-slate-500 text-xs">
                Nenhum dispositivo encontrado neste filtro.
              </div>
            ) : (
              <table className="w-full text-left text-xs font-mono">
                <thead className="bg-slate-950/40 text-slate-400 uppercase text-[10px] tracking-wider border-b border-slate-800 sticky top-0 backdrop-blur">
                  <tr>
                    <th className="px-4 py-2 w-8">
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
                    <th className="px-4 py-2">Dispositivo / Nome</th>
                    <th className="px-4 py-2">Tipo de Equipamento</th>
                    <th className="px-4 py-2">Modelo</th>
                    <th className="px-4 py-2">IP : Porta</th>
                    <th className="px-4 py-2">Status</th>
                    <th className="px-4 py-2 text-right">Ações Rápidas</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-slate-800/50">
                  {filteredDevices.map((dev) => {
                    const isSelected = selectedIps.has(dev.ip);
                    const isAdded = dev.is_already_added;

                    return (
                      <tr
                        key={dev.ip}
                        className={`hover:bg-slate-800/40 transition ${
                          isSelected ? 'bg-sky-500/5' : ''
                        }`}
                      >
                        <td className="px-4 py-2.5">
                          <input
                            type="checkbox"
                            checked={isSelected}
                            disabled={isAdded}
                            onChange={() => toggleSelect(dev.ip)}
                            className="rounded border-slate-700 bg-slate-900 text-sky-500 focus:ring-sky-500 h-3.5 w-3.5 cursor-pointer disabled:opacity-30"
                          />
                        </td>
                        <td className="px-4 py-2.5 font-sans">
                          <div className="font-bold text-white leading-tight">{dev.name}</div>
                          <div className="text-[10px] text-sky-400/80 font-mono mt-0.5">
                            {dev.brand}
                          </div>
                        </td>
                        <td className="px-4 py-2.5">
                          <span
                            className={`inline-flex items-center gap-1 px-2 py-0.5 rounded-md text-[10px] font-semibold border ${getTypeBadgeClass(
                              dev.device_type
                            )}`}
                          >
                            {getTypeIcon(dev.device_type)}
                            <span>{dev.device_type_label}</span>
                          </span>
                        </td>
                        <td className="px-4 py-2.5 text-slate-300 font-semibold">
                          {dev.hardware_model}
                        </td>
                        <td className="px-4 py-2.5 text-slate-200">
                          {dev.ip}:{dev.rtsp_port}
                        </td>
                        <td className="px-4 py-2.5">
                          {isAdded ? (
                            <span className="inline-flex items-center gap-1 text-[10px] font-semibold text-emerald-400">
                              <CheckCircle2 className="h-3 w-3" />
                              Cadastrada
                            </span>
                          ) : (
                            <span className="inline-flex items-center gap-1 text-[10px] font-semibold text-sky-400">
                              <Radio className="h-3 w-3" />
                              Disponível
                            </span>
                          )}
                        </td>
                        <td className="px-4 py-2.5 text-right font-sans">
                          {!isAdded ? (
                            <button
                              onClick={() => {
                                onAddSingle({
                                  name: dev.name,
                                  host: dev.ip,
                                  username: 'admin',
                                  rtsp_port: dev.rtsp_port,
                                  stream_profile: 'main',
                                });
                              }}
                              className="px-2.5 py-1 rounded bg-sky-600 hover:bg-sky-500 text-white text-xs font-semibold inline-flex items-center gap-1 transition shadow"
                              title="Adicionar com 1 clique"
                            >
                              <Plus className="h-3 w-3" />
                              <span>Adicionar</span>
                            </button>
                          ) : (
                            <span className="text-[11px] text-slate-500 italic">No banco</span>
                          )}
                        </td>
                      </tr>
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
                <span>Marque as caixas para cadastrar múltiplos dispositivos com uma única senha</span>
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
