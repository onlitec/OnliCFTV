import React, { useState, useMemo } from 'react';
import {
  Plus,
  Play,
  Square,
  Edit2,
  Trash2,
  Activity,
  Radio,
  Loader2,
  Eye,
  Camera as CameraIcon,
  Search,
  AlertTriangle,
  LayoutList,
  LayoutGrid,
  Copy,
  Check,
} from 'lucide-react';
import type {
  Camera,
  CameraStreamStatus,
  DiscoveredDevice,
  CreateCameraInput,
} from '@/types';
import { DiscoveryPanel } from '@/components/DiscoveryPanel';
import { api } from '@/services/api';

interface DashboardPageProps {
  viewMode?: 'discovery' | 'cameras';
  cameras: Camera[];
  streamStatuses: Record<string, CameraStreamStatus>;
  discoveredDevices: DiscoveredDevice[];
  isScanning: boolean;
  onRefreshScan: (interfaceName?: string) => void;
  onAddCamera: () => void;
  onEditCamera: (cam: Camera) => void;
  onDeleteCamera: (id: string) => void;
  onStartStream: (id: string) => void;
  onStopStream: (id: string) => void;
  onOpenLiveCamera: (cameraId: string) => void;
  onAddSingleFromDiscovery: (prefill: CreateCameraInput) => void;
  onDataChanged: () => void;
}

export const DashboardPage: React.FC<DashboardPageProps> = ({
  viewMode = 'discovery',
  cameras,
  streamStatuses,
  discoveredDevices,
  isScanning,
  onRefreshScan,
  onAddCamera,
  onEditCamera,
  onDeleteCamera,
  onStartStream,
  onStopStream,
  onOpenLiveCamera,
  onAddSingleFromDiscovery,
  onDataChanged,
}) => {
  const [testingId, setTestingId] = useState<string | null>(null);
  const [testResult, setTestResult] = useState<{ id: string; success: boolean; msg: string } | null>(
    null
  );

  // List view state
  const [displayMode, setDisplayMode] = useState<'list' | 'grid'>('list');
  const [searchQuery, setSearchQuery] = useState('');
  const [statusFilter, setStatusFilter] = useState<'all' | 'online' | 'offline'>('all');
  const [selectedCameraIds, setSelectedCameraIds] = useState<Set<string>>(new Set());
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const [isDeletingBatch, setIsDeletingBatch] = useState(false);
  const [deleteConfirmModal, setDeleteConfirmModal] = useState<{
    isOpen: boolean;
    mode: 'selected' | 'all' | 'single';
    targetId?: string;
    targetName?: string;
    count: number;
  }>({
    isOpen: false,
    mode: 'selected',
    count: 0,
  });

  const handleTestExisting = async (e: React.MouseEvent, id: string) => {
    e.stopPropagation();
    setTestingId(id);
    setTestResult(null);
    try {
      const res = await api.testExistingCamera(id);
      setTestResult({
        id,
        success: res.success,
        msg: res.success
          ? `OK: ${res.codec || 'Vídeo'} ${res.resolution || ''} (${res.latency_ms || 0}ms)`
          : res.message,
      });
    } catch (err: any) {
      setTestResult({
        id,
        success: false,
        msg: err?.toString() || 'Erro no teste',
      });
    } finally {
      setTestingId(null);
    }
  };

  const handleCopy = (text: string, id: string) => {
    navigator.clipboard.writeText(text);
    setCopiedId(id);
    setTimeout(() => setCopiedId(null), 2000);
  };

  // Filtered cameras
  const filteredCameras = useMemo(() => {
    return cameras.filter((cam) => {
      const query = searchQuery.toLowerCase().trim();
      const matchesQuery =
        !query ||
        cam.name.toLowerCase().includes(query) ||
        cam.host.toLowerCase().includes(query) ||
        cam.username.toLowerCase().includes(query) ||
        cam.rtsp_url.toLowerCase().includes(query);

      const status = streamStatuses[cam.id];
      const isOnline = status?.state === 'online';

      if (!matchesQuery) return false;
      if (statusFilter === 'online') return isOnline;
      if (statusFilter === 'offline') return !isOnline;
      return true;
    });
  }, [cameras, searchQuery, statusFilter, streamStatuses]);

  // Toggle single selection
  const toggleSelectCamera = (id: string) => {
    setSelectedCameraIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  };

  // Toggle select all filtered
  const toggleSelectAllFiltered = () => {
    if (filteredCameras.length === 0) return;
    const allFilteredSelected = filteredCameras.every((c) => selectedCameraIds.has(c.id));
    if (allFilteredSelected) {
      setSelectedCameraIds((prev) => {
        const next = new Set(prev);
        filteredCameras.forEach((c) => next.delete(c.id));
        return next;
      });
    } else {
      setSelectedCameraIds((prev) => {
        const next = new Set(prev);
        filteredCameras.forEach((c) => next.add(c.id));
        return next;
      });
    }
  };

  // Execute batch or all deletion
  const handleConfirmDelete = async () => {
    setIsDeletingBatch(true);
    try {
      if (deleteConfirmModal.mode === 'all') {
        await api.deleteAllCameras();
        setSelectedCameraIds(new Set());
      } else if (deleteConfirmModal.mode === 'selected') {
        const ids = Array.from(selectedCameraIds);
        if (ids.length > 0) {
          await api.deleteCamerasBatch(ids);
          setSelectedCameraIds(new Set());
        }
      } else if (deleteConfirmModal.mode === 'single' && deleteConfirmModal.targetId) {
        onDeleteCamera(deleteConfirmModal.targetId);
        setSelectedCameraIds((prev) => {
          const next = new Set(prev);
          next.delete(deleteConfirmModal.targetId!);
          return next;
        });
      }
      onDataChanged();
    } catch (err: any) {
      alert(`Erro ao remover dispositivos: ${err?.toString()}`);
    } finally {
      setIsDeletingBatch(false);
      setDeleteConfirmModal({ isOpen: false, mode: 'selected', count: 0 });
    }
  };

  // 1. DISCOVERY & COMMISSIONING VIEW (Full Screen Commissioning Center)
  if (viewMode === 'discovery') {
    return (
      <div className="p-4 h-full flex flex-col select-none overflow-hidden">
        <DiscoveryPanel
          discoveredDevices={discoveredDevices}
          isScanning={isScanning}
          onRefreshScan={onRefreshScan}
          onAdded={onDataChanged}
          onAddSingle={onAddSingleFromDiscovery}
          onOpenManualAdd={onAddCamera}
        />
      </div>
    );
  }

  const onlineCount = cameras.filter((c) => streamStatuses[c.id]?.state === 'online').length;
  const offlineCount = cameras.length - onlineCount;

  // 2. REGISTERED CAMERAS VIEW (High Density List & Management Center)
  return (
    <div className="p-4 h-full flex flex-col select-none overflow-hidden space-y-3">
      {/* Top Header Card */}
      <div className="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-xl p-3.5 flex flex-wrap items-center justify-between gap-3 shrink-0 shadow-sm transition-colors">
        <div className="flex items-center gap-3">
          <div className="h-9 w-9 rounded-xl bg-sky-50 dark:bg-sky-500/15 border border-sky-200 dark:border-sky-500/30 flex items-center justify-center text-sky-600 dark:text-sky-400">
            <CameraIcon className="h-5 w-5" />
          </div>
          <div>
            <div className="flex items-center gap-2">
              <h2 className="text-base font-bold text-slate-800 dark:text-white tracking-tight">
                Dispositivos Cadastrados
              </h2>
              <span className="px-2 py-0.5 rounded-md text-[11px] font-mono font-bold bg-sky-50 dark:bg-sky-500/15 text-sky-700 dark:text-sky-300 border border-sky-200 dark:border-sky-500/30">
                {cameras.length} no Total
              </span>
              <span className="px-2 py-0.5 rounded-md text-[11px] font-mono font-bold bg-emerald-50 dark:bg-emerald-500/15 text-emerald-700 dark:text-emerald-300 border border-emerald-200 dark:border-emerald-500/30">
                {onlineCount} Online
              </span>
            </div>
            <p className="text-xs text-slate-500 dark:text-slate-400 font-sans">
              Gerencie parâmetros de stream, execute testes de rede ou remova dispositivos em lote.
            </p>
          </div>
        </div>

        {/* Global Action Buttons */}
        <div className="flex items-center gap-2">
          {cameras.length > 0 && (
            <button
              onClick={() =>
                setDeleteConfirmModal({
                  isOpen: true,
                  mode: 'all',
                  count: cameras.length,
                })
              }
              className="px-3 py-1.5 rounded-lg bg-rose-50 dark:bg-rose-500/15 hover:bg-rose-100 dark:hover:bg-rose-500/25 text-rose-600 dark:text-rose-300 border border-rose-200 dark:border-rose-500/30 text-xs font-semibold flex items-center gap-1.5 transition shadow-sm"
              title="Remover todas as câmeras cadastradas de uma só vez"
            >
              <Trash2 className="h-3.5 w-3.5" />
              <span>Remover Todas</span>
            </button>
          )}

          <button
            onClick={() => onOpenLiveCamera('')}
            className="px-3 py-1.5 rounded-lg bg-slate-100 dark:bg-slate-800 hover:bg-slate-200 dark:hover:bg-slate-700 text-slate-700 dark:text-slate-200 border border-slate-200 dark:border-slate-700 text-xs font-semibold flex items-center gap-1.5 transition shadow-sm"
          >
            <Radio className="h-3.5 w-3.5 text-sky-600 dark:text-sky-400" />
            <span>Mosaico Geral</span>
          </button>

          <button
            onClick={onAddCamera}
            className="px-3.5 py-1.5 rounded-lg bg-sky-600 hover:bg-sky-500 text-white text-xs font-bold shadow-md flex items-center gap-1.5 transition"
          >
            <Plus className="h-3.5 w-3.5" />
            <span>Cadastrar Manual</span>
          </button>
        </div>
      </div>

      {/* Filter Toolbar */}
      <div className="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-xl px-3.5 py-2 flex flex-wrap items-center justify-between gap-3 shrink-0 shadow-sm transition-colors">
        {/* Search Input */}
        <div className="relative flex-1 min-w-[240px] max-w-md">
          <Search className="h-3.5 w-3.5 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Buscar por nome, IP, porta ou URI..."
            className="w-full bg-slate-50 dark:bg-slate-950 border border-slate-200 dark:border-slate-800 rounded-lg pl-9 pr-3 py-1.5 text-xs text-slate-800 dark:text-slate-200 focus:outline-none focus:border-sky-500 font-sans transition"
          />
        </div>

        {/* Status Pills & Layout Switch */}
        <div className="flex items-center gap-2">
          <div className="flex items-center gap-1 bg-slate-100 dark:bg-slate-950 border border-slate-200 dark:border-slate-800 rounded-lg p-0.5">
            <button
              onClick={() => setStatusFilter('all')}
              className={`px-2.5 py-1 rounded text-xs font-semibold transition ${
                statusFilter === 'all'
                  ? 'bg-sky-600 text-white shadow-sm'
                  : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white'
              }`}
            >
              Todas ({cameras.length})
            </button>
            <button
              onClick={() => setStatusFilter('online')}
              className={`px-2.5 py-1 rounded text-xs font-semibold transition ${
                statusFilter === 'online'
                  ? 'bg-emerald-600 text-white shadow-sm'
                  : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white'
              }`}
            >
              Online ({onlineCount})
            </button>
            <button
              onClick={() => setStatusFilter('offline')}
              className={`px-2.5 py-1 rounded text-xs font-semibold transition ${
                statusFilter === 'offline'
                  ? 'bg-slate-700 text-white shadow-sm'
                  : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white'
              }`}
            >
              Offline ({offlineCount})
            </button>
          </div>

          {/* List / Grid toggle */}
          <div className="flex items-center bg-slate-100 dark:bg-slate-950 border border-slate-200 dark:border-slate-800 rounded-lg p-0.5">
            <button
              onClick={() => setDisplayMode('list')}
              className={`p-1 rounded transition ${
                displayMode === 'list'
                  ? 'bg-white dark:bg-slate-800 text-sky-600 dark:text-sky-400 shadow-sm'
                  : 'text-slate-400 hover:text-slate-600 dark:hover:text-slate-300'
              }`}
              title="Visualização em Lista"
            >
              <LayoutList className="h-4 w-4" />
            </button>
            <button
              onClick={() => setDisplayMode('grid')}
              className={`p-1 rounded transition ${
                displayMode === 'grid'
                  ? 'bg-white dark:bg-slate-800 text-sky-600 dark:text-sky-400 shadow-sm'
                  : 'text-slate-400 hover:text-slate-600 dark:hover:text-slate-300'
              }`}
              title="Visualização em Grade de Cards"
            >
              <LayoutGrid className="h-4 w-4" />
            </button>
          </div>
        </div>
      </div>

      {/* Main Content Area */}
      <div className="bg-white dark:bg-slate-950 border border-slate-200 dark:border-slate-800 rounded-xl overflow-hidden shadow-sm flex-1 min-h-0 flex flex-col transition-colors">
        {cameras.length === 0 ? (
          <div className="flex-1 flex items-center justify-center p-12 text-center">
            <div className="space-y-3 max-w-md">
              <div className="h-12 w-12 rounded-2xl bg-sky-50 dark:bg-sky-500/15 text-sky-600 dark:text-sky-400 flex items-center justify-center mx-auto">
                <CameraIcon className="h-6 w-6" />
              </div>
              <h4 className="text-base font-bold text-slate-800 dark:text-white">Nenhuma câmera cadastrada</h4>
              <p className="text-xs text-slate-500 dark:text-slate-400">
                Utilize a aba de Descoberta para localizar e adicionar dispositivos da rede local com 1 clique.
              </p>
            </div>
          </div>
        ) : filteredCameras.length === 0 ? (
          <div className="flex-1 flex items-center justify-center p-12 text-center text-xs text-slate-500">
            Nenhuma câmera encontrada com os filtros selecionados.
          </div>
        ) : displayMode === 'list' ? (
          /* HIGH DENSITY TABLE VIEW (Padronizado White & Dark) */
          <div className="flex-1 min-h-0 overflow-y-auto">
            <table className="w-full text-left text-xs font-mono border-collapse">
              <thead className="bg-slate-50 dark:bg-slate-900 text-slate-600 dark:text-slate-400 uppercase text-[10px] tracking-wider border-b border-slate-200 dark:border-slate-800 sticky top-0 backdrop-blur z-10 shadow-sm">
                <tr>
                  <th className="px-3 py-2.5 w-8 text-center">
                    <input
                      type="checkbox"
                      checked={
                        filteredCameras.length > 0 &&
                        filteredCameras.every((c) => selectedCameraIds.has(c.id))
                      }
                      onChange={toggleSelectAllFiltered}
                      className="rounded border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-950 text-sky-600 focus:ring-sky-500 h-3.5 w-3.5 cursor-pointer"
                    />
                  </th>
                  <th className="px-2.5 py-2.5 w-10 text-center text-slate-400 dark:text-slate-500">Nº</th>
                  <th className="px-3 py-2.5 w-24">Status</th>
                  <th className="px-3 py-2.5">Nome do Dispositivo</th>
                  <th className="px-3 py-2.5">Endereço IP & Porta</th>
                  <th className="px-3 py-2.5">Perfil</th>
                  <th className="px-3 py-2.5">Stream URI</th>
                  <th className="px-3 py-2.5">Telemetria (FPS / Res.)</th>
                  <th className="px-3 py-2.5 text-right">Ações</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-100 dark:divide-slate-800/60 bg-white dark:bg-slate-950">
                {filteredCameras.map((cam, idx) => {
                  const isSelected = selectedCameraIds.has(cam.id);
                  const status = streamStatuses[cam.id];
                  const isOnline = status?.state === 'online';
                  const isConnecting = status?.state === 'connecting';
                  const isTestingThis = testingId === cam.id;
                  const thisTestResult = testResult?.id === cam.id ? testResult : null;

                  return (
                    <tr
                      key={cam.id}
                      className={`hover:bg-sky-50/60 dark:hover:bg-slate-900/60 transition ${
                        isSelected ? 'bg-sky-50 dark:bg-sky-500/10' : ''
                      }`}
                    >
                      {/* Checkbox */}
                      <td className="px-3 py-2 text-center">
                        <input
                          type="checkbox"
                          checked={isSelected}
                          onChange={() => toggleSelectCamera(cam.id)}
                          className="rounded border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-950 text-sky-600 focus:ring-sky-500 h-3.5 w-3.5 cursor-pointer"
                        />
                      </td>

                      {/* Nº */}
                      <td className="px-2.5 py-2 text-center text-slate-400 dark:text-slate-500 font-mono text-[11px]">
                        {idx + 1}
                      </td>

                      {/* Status */}
                      <td className="px-3 py-2">
                        {isOnline ? (
                          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-semibold bg-emerald-50 dark:bg-emerald-500/15 text-emerald-700 dark:text-emerald-300 border border-emerald-200 dark:border-emerald-500/30">
                            <span className="h-1.5 w-1.5 rounded-full bg-emerald-500 animate-pulse" />
                            ONLINE
                          </span>
                        ) : isConnecting ? (
                          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-semibold bg-amber-50 dark:bg-amber-500/15 text-amber-700 dark:text-amber-300 border border-amber-200 dark:border-amber-500/30">
                            <Loader2 className="h-2.5 w-2.5 animate-spin" />
                            CONECTANDO
                          </span>
                        ) : (
                          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-semibold bg-slate-100 dark:bg-slate-800 text-slate-500 dark:text-slate-400 border border-slate-200 dark:border-slate-700">
                            INATIVA
                          </span>
                        )}
                      </td>

                      {/* Nome */}
                      <td className="px-3 py-2 font-sans font-bold text-slate-800 dark:text-slate-200">
                        <div
                          onClick={() => onOpenLiveCamera(cam.id)}
                          className="cursor-pointer hover:text-sky-600 dark:hover:text-sky-400 transition"
                          title="Abrir no player ao vivo"
                        >
                          {cam.name}
                        </div>
                      </td>

                      {/* IP & Porta */}
                      <td className="px-3 py-2">
                        <div className="flex items-center gap-1 font-bold text-sky-600 dark:text-sky-400 text-xs">
                          <span>{cam.host}:{cam.rtsp_port}</span>
                          <button
                            onClick={() => handleCopy(cam.host, cam.id)}
                            className="text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 p-0.5"
                            title={copiedId === cam.id ? 'Copiado!' : 'Copiar IP'}
                          >
                            {copiedId === cam.id ? (
                              <Check className="h-3 w-3 text-emerald-500" />
                            ) : (
                              <Copy className="h-3 w-3" />
                            )}
                          </button>
                        </div>
                      </td>

                      {/* Perfil */}
                      <td className="px-3 py-2">
                        <span className="px-2 py-0.5 rounded text-[10px] font-mono font-bold bg-slate-100 dark:bg-slate-900 text-slate-700 dark:text-slate-300 border border-slate-200 dark:border-slate-800">
                          {cam.stream_profile.toUpperCase()}
                        </span>
                      </td>

                      {/* URI */}
                      <td className="px-3 py-2">
                        <div className="max-w-[200px] truncate text-[11px] text-slate-500 dark:text-slate-400" title={cam.rtsp_url}>
                          {cam.rtsp_url}
                        </div>
                      </td>

                      {/* Telemetria / Teste */}
                      <td className="px-3 py-2">
                        {thisTestResult ? (
                          <span className={`text-[10px] font-semibold ${thisTestResult.success ? 'text-emerald-600 dark:text-emerald-400' : 'text-rose-600 dark:text-rose-400'}`}>
                            {thisTestResult.msg}
                          </span>
                        ) : status && isOnline ? (
                          <div className="text-[11px] text-emerald-600 dark:text-emerald-400 font-bold">
                            {status.fps} fps • {status.resolution || '1080p'}
                          </div>
                        ) : (
                          <span className="text-[10px] text-slate-400 dark:text-slate-500">
                            Sem stream ativo
                          </span>
                        )}
                      </td>

                      {/* Ações */}
                      <td className="px-3 py-2 text-right font-sans">
                        <div className="flex items-center justify-end gap-1.5">
                          {/* Live view */}
                          <button
                            onClick={() => onOpenLiveCamera(cam.id)}
                            className="px-2 py-1 rounded bg-sky-50 dark:bg-sky-500/20 hover:bg-sky-100 dark:hover:bg-sky-500/35 text-sky-600 dark:text-sky-300 border border-sky-200 dark:border-sky-500/40 text-xs font-bold inline-flex items-center gap-1 transition shadow-sm"
                            title="Visualizar Câmera Ao Vivo"
                          >
                            <Eye className="h-3.5 w-3.5" />
                            <span>Ver</span>
                          </button>

                          {/* Connect / Stop Stream */}
                          {isOnline ? (
                            <button
                              onClick={() => onStopStream(cam.id)}
                              className="p-1.5 rounded bg-rose-50 dark:bg-rose-500/15 hover:bg-rose-100 dark:hover:bg-rose-500/25 text-rose-600 dark:text-rose-300 border border-rose-200 dark:border-rose-500/30 transition"
                              title="Interromper Stream"
                            >
                              <Square className="h-3.5 w-3.5 fill-current" />
                            </button>
                          ) : (
                            <button
                              onClick={() => onStartStream(cam.id)}
                              className="p-1.5 rounded bg-emerald-50 dark:bg-emerald-500/15 hover:bg-emerald-100 dark:hover:bg-emerald-500/25 text-emerald-600 dark:text-emerald-300 border border-emerald-200 dark:border-emerald-500/30 transition"
                              title="Iniciar Stream"
                            >
                              <Play className="h-3.5 w-3.5 fill-current" />
                            </button>
                          )}

                          {/* Test */}
                          <button
                            onClick={(e) => handleTestExisting(e, cam.id)}
                            disabled={isTestingThis}
                            className="p-1.5 rounded bg-slate-100 dark:bg-slate-800 hover:bg-slate-200 dark:hover:bg-slate-700 text-slate-600 dark:text-slate-300 border border-slate-200 dark:border-slate-700 transition"
                            title="Testar Conexão RTSP"
                          >
                            {isTestingThis ? (
                              <Loader2 className="h-3.5 w-3.5 animate-spin text-sky-600 dark:text-sky-400" />
                            ) : (
                              <Activity className="h-3.5 w-3.5 text-sky-600 dark:text-sky-400" />
                            )}
                          </button>

                          {/* Edit */}
                          <button
                            onClick={() => onEditCamera(cam)}
                            className="p-1.5 rounded bg-slate-100 dark:bg-slate-800 hover:bg-slate-200 dark:hover:bg-slate-700 text-slate-600 dark:text-slate-300 border border-slate-200 dark:border-slate-700 transition"
                            title="Editar Dados da Câmera"
                          >
                            <Edit2 className="h-3.5 w-3.5" />
                          </button>

                          {/* Delete */}
                          <button
                            onClick={() =>
                              setDeleteConfirmModal({
                                isOpen: true,
                                mode: 'single',
                                targetId: cam.id,
                                targetName: cam.name,
                                count: 1,
                              })
                            }
                            className="p-1.5 rounded bg-rose-50 dark:bg-rose-500/10 hover:bg-rose-100 dark:hover:bg-rose-500/20 text-rose-600 dark:text-rose-400 border border-rose-200 dark:border-rose-500/30 transition"
                            title="Excluir Câmera"
                          >
                            <Trash2 className="h-3.5 w-3.5" />
                          </button>
                        </div>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        ) : (
          /* GRID VIEW */
          <div className="flex-1 min-h-0 overflow-y-auto p-4">
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
              {filteredCameras.map((cam) => {
                const status = streamStatuses[cam.id];
                const isOnline = status?.state === 'online';
                const isConnecting = status?.state === 'connecting';
                const isSelected = selectedCameraIds.has(cam.id);

                return (
                  <div
                    key={cam.id}
                    className={`bg-white dark:bg-slate-900 border rounded-xl p-4 flex flex-col justify-between transition shadow-sm relative ${
                      isSelected
                        ? 'border-sky-500 ring-2 ring-sky-500/20'
                        : 'border-slate-200 dark:border-slate-800 hover:border-slate-300 dark:hover:border-slate-700'
                    }`}
                  >
                    <div className="flex items-start justify-between gap-2 mb-3">
                      <div className="flex items-center gap-2.5">
                        <input
                          type="checkbox"
                          checked={isSelected}
                          onChange={() => toggleSelectCamera(cam.id)}
                          className="rounded border-slate-300 dark:border-slate-700 text-sky-600 h-4 w-4 cursor-pointer"
                        />
                        <div>
                          <h4 className="text-sm font-bold text-slate-800 dark:text-white leading-tight">
                            {cam.name}
                          </h4>
                          <p className="text-xs font-mono text-sky-600 dark:text-sky-400 mt-0.5">
                            {cam.host}:{cam.rtsp_port}
                          </p>
                        </div>
                      </div>

                      {isOnline ? (
                        <span className="px-2 py-0.5 rounded-full text-[10px] font-semibold bg-emerald-50 dark:bg-emerald-500/15 text-emerald-700 dark:text-emerald-300 border border-emerald-200 dark:border-emerald-500/30">
                          Online
                        </span>
                      ) : isConnecting ? (
                        <span className="px-2 py-0.5 rounded-full text-[10px] font-semibold bg-amber-50 dark:bg-amber-500/15 text-amber-700 dark:text-amber-300 border border-amber-200 dark:border-amber-500/30">
                          Conectando
                        </span>
                      ) : (
                        <span className="px-2 py-0.5 rounded-full text-[10px] font-semibold bg-slate-100 dark:bg-slate-800 text-slate-500 dark:text-slate-400 border border-slate-200 dark:border-slate-700">
                          Inativa
                        </span>
                      )}
                    </div>

                    <div className="mb-3">
                      <button
                        onClick={() => onOpenLiveCamera(cam.id)}
                        className="w-full py-2 rounded-lg bg-sky-50 dark:bg-sky-500/20 hover:bg-sky-100 dark:hover:bg-sky-500/30 text-sky-600 dark:text-sky-300 border border-sky-200 dark:border-sky-500/40 text-xs font-bold flex items-center justify-center gap-2 transition"
                      >
                        <Eye className="h-4 w-4" />
                        <span>Abrir Imagem Ao Vivo</span>
                      </button>
                    </div>

                    <div className="pt-2 border-t border-slate-100 dark:border-slate-800 flex items-center justify-between text-xs">
                      <div className="flex items-center gap-1">
                        <button
                          onClick={() => onEditCamera(cam)}
                          className="p-1 rounded text-slate-400 hover:text-slate-700 dark:hover:text-white"
                        >
                          <Edit2 className="h-3.5 w-3.5" />
                        </button>
                        <button
                          onClick={() =>
                            setDeleteConfirmModal({
                              isOpen: true,
                              mode: 'single',
                              targetId: cam.id,
                              targetName: cam.name,
                              count: 1,
                            })
                          }
                          className="p-1 rounded text-slate-400 hover:text-rose-600"
                        >
                          <Trash2 className="h-3.5 w-3.5" />
                        </button>
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        )}

        {/* FLOATING BOTTOM BATCH ACTION BAR */}
        {selectedCameraIds.size > 0 && (
          <div className="px-5 py-2.5 bg-slate-50 dark:bg-slate-900 border-t border-slate-200 dark:border-slate-800 flex items-center justify-between text-xs shrink-0 transition-colors animate-in slide-in-from-bottom duration-150">
            <div className="flex items-center gap-3 text-slate-700 dark:text-slate-300">
              <span className="font-bold">
                Selecionadas:{' '}
                <strong className="text-sky-600 dark:text-sky-400 font-mono text-sm">
                  {selectedCameraIds.size}
                </strong>{' '}
                de {cameras.length}
              </span>
              <button
                onClick={() => setSelectedCameraIds(new Set())}
                className="text-xs text-slate-500 hover:text-slate-700 dark:hover:text-slate-300 underline"
              >
                Limpar seleção
              </button>
            </div>

            <div className="flex items-center gap-2">
              <button
                onClick={() =>
                  setDeleteConfirmModal({
                    isOpen: true,
                    mode: 'selected',
                    count: selectedCameraIds.size,
                  })
                }
                className="px-4 py-1.5 rounded-lg bg-rose-600 hover:bg-rose-500 text-white font-bold text-xs flex items-center gap-1.5 transition shadow-sm shadow-rose-500/20"
              >
                <Trash2 className="h-3.5 w-3.5" />
                <span>Excluir Selecionadas ({selectedCameraIds.size})</span>
              </button>
            </div>
          </div>
        )}
      </div>

      {/* CONFIRMATION DELETION MODAL */}
      {deleteConfirmModal.isOpen && (
        <div className="fixed inset-0 z-50 bg-black/60 backdrop-blur-sm flex items-center justify-center p-4">
          <div className="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-2xl shadow-2xl max-w-md w-full p-6 space-y-4 animate-in fade-in zoom-in-95 duration-150 transition-colors">
            <div className="flex items-center gap-3 text-rose-600 dark:text-rose-400">
              <div className="h-10 w-10 rounded-xl bg-rose-50 dark:bg-rose-500/20 border border-rose-200 dark:border-rose-500/30 flex items-center justify-center">
                <AlertTriangle className="h-5 w-5" />
              </div>
              <div>
                <h3 className="text-base font-bold text-slate-800 dark:text-white">
                  Confirmar Exclusão
                </h3>
                <p className="text-xs text-slate-500 dark:text-slate-400">
                  Esta ação removerá as configurações do banco de dados local.
                </p>
              </div>
            </div>

            <div className="p-3.5 rounded-xl bg-slate-50 dark:bg-slate-950 border border-slate-200 dark:border-slate-800 text-xs text-slate-700 dark:text-slate-300">
              {deleteConfirmModal.mode === 'all' ? (
                <span>
                  Tem certeza que deseja <strong>remover TODAS as {deleteConfirmModal.count} câmeras</strong> cadastradas no sistema?
                </span>
              ) : deleteConfirmModal.mode === 'selected' ? (
                <span>
                  Tem certeza que deseja <strong>remover as {deleteConfirmModal.count} câmeras selecionadas</strong>?
                </span>
              ) : (
                <span>
                  Tem certeza que deseja remover a câmera <strong>"{deleteConfirmModal.targetName}"</strong>?
                </span>
              )}
            </div>

            <div className="flex items-center justify-end gap-2.5 pt-2">
              <button
                type="button"
                disabled={isDeletingBatch}
                onClick={() => setDeleteConfirmModal({ isOpen: false, mode: 'selected', count: 0 })}
                className="px-4 py-2 rounded-xl bg-slate-100 dark:bg-slate-800 hover:bg-slate-200 dark:hover:bg-slate-700 text-slate-700 dark:text-slate-300 text-xs font-semibold transition"
              >
                Cancelar
              </button>
              <button
                type="button"
                disabled={isDeletingBatch}
                onClick={handleConfirmDelete}
                className="px-5 py-2 rounded-xl bg-rose-600 hover:bg-rose-500 text-white text-xs font-bold transition flex items-center gap-1.5 shadow-md shadow-rose-600/20 disabled:opacity-50"
              >
                {isDeletingBatch ? (
                  <>
                    <Loader2 className="h-3.5 w-3.5 animate-spin" />
                    <span>Excluindo...</span>
                  </>
                ) : (
                  <>
                    <Trash2 className="h-3.5 w-3.5" />
                    <span>Confirmar e Excluir</span>
                  </>
                )}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
