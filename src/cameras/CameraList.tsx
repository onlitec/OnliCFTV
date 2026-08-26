import React, { useState } from 'react';
import {
  Plus,
  Play,
  Square,
  Edit2,
  Trash2,
  Activity,
  Radio,
  Loader2,
  Search,
  Sparkles,
} from 'lucide-react';
import type { Camera, CameraStreamStatus, DiscoveredDevice } from '@/types';
import { api } from '@/services/api';

interface CameraListProps {
  cameras: Camera[];
  streamStatuses: Record<string, CameraStreamStatus>;
  discoveredDevices: DiscoveredDevice[];
  onOpenDiscovery: () => void;
  onAddCamera: () => void;
  onEditCamera: (cam: Camera) => void;
  onDeleteCamera: (id: string) => void;
  onStartStream: (id: string) => void;
  onStopStream: (id: string) => void;
  onViewLive: () => void;
}

export const CameraList: React.FC<CameraListProps> = ({
  cameras,
  streamStatuses,
  discoveredDevices,
  onOpenDiscovery,
  onAddCamera,
  onEditCamera,
  onDeleteCamera,
  onStartStream,
  onStopStream,
  onViewLive,
}) => {
  const [testingId, setTestingId] = useState<string | null>(null);
  const [testResult, setTestResult] = useState<{ id: string; success: boolean; msg: string } | null>(null);

  const newDiscoveredCount = discoveredDevices.filter((d) => !d.is_already_added).length;

  const handleTestExisting = async (id: string) => {
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
    } catch (e: any) {
      setTestResult({
        id,
        success: false,
        msg: e?.toString() || 'Erro no teste',
      });
    } finally {
      setTestingId(null);
    }
  };

  return (
    <div className="p-6 space-y-6 select-none">
      {/* Smart Discovery Notification Banner */}
      {newDiscoveredCount > 0 && (
        <div className="bg-gradient-to-r from-sky-950/70 via-slate-900 to-sky-950/70 border border-sky-500/40 rounded-xl p-4 flex items-center justify-between shadow-lg shadow-sky-950/30 animate-in fade-in slide-in-from-top-2 duration-300">
          <div className="flex items-center gap-3">
            <div className="h-9 w-9 rounded-lg bg-sky-500/20 border border-sky-500/30 flex items-center justify-center text-sky-400">
              <Sparkles className="h-5 w-5 animate-pulse" />
            </div>
            <div>
              <h4 className="text-sm font-bold text-white flex items-center gap-2">
                {newDiscoveredCount} novo(s) dispositivo(s) localizado(s) na rede local!
              </h4>
              <p className="text-xs text-slate-300">
                Adicione individualmente ou todos de uma vez selecionando e informando a senha em lote.
              </p>
            </div>
          </div>
          <button
            onClick={onOpenDiscovery}
            className="px-4 py-2 rounded-lg bg-sky-600 hover:bg-sky-500 text-white text-xs font-bold shadow-md shadow-sky-950 flex items-center gap-1.5 transition shrink-0"
          >
            <Search className="h-3.5 w-3.5" />
            <span>Ver Dispositivos ({newDiscoveredCount})</span>
          </button>
        </div>
      )}

      {/* Header and Action Buttons */}
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-xl font-bold text-white tracking-tight">Gerenciamento de Câmeras</h3>
          <p className="text-xs text-slate-400 mt-1">
            Cadastre, teste e gerencie suas câmeras IP, NVRs e canais Hikvision via RTSP.
          </p>
        </div>
        <div className="flex items-center gap-2.5">
          <button
            onClick={onOpenDiscovery}
            className="px-3.5 py-2 rounded-lg bg-slate-800 hover:bg-slate-700 text-sky-400 hover:text-sky-300 text-sm font-semibold border border-slate-700 flex items-center gap-2 transition"
          >
            <Search className="h-4 w-4" />
            <span>Buscar na Rede</span>
          </button>
          <button
            onClick={onViewLive}
            className="px-3.5 py-2 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-200 text-sm font-medium border border-slate-700 flex items-center gap-2 transition"
          >
            <Radio className="h-4 w-4 text-sky-400" />
            <span>Mosaico Ao Vivo</span>
          </button>
          <button
            onClick={onAddCamera}
            className="px-4 py-2 rounded-lg bg-sky-600 hover:bg-sky-500 text-white text-sm font-semibold shadow-md shadow-sky-950 flex items-center gap-2 transition"
          >
            <Plus className="h-4 w-4" />
            <span>Adicionar Câmera</span>
          </button>
        </div>
      </div>

      {/* Camera Cards / Table */}
      {cameras.length === 0 ? (
        <div className="bg-slate-900/60 border border-dashed border-slate-800 rounded-xl p-12 text-center">
          <div className="h-12 w-12 rounded-full bg-slate-800 flex items-center justify-center mx-auto text-slate-400 mb-3">
            <Radio className="h-6 w-6" />
          </div>
          <h4 className="text-base font-bold text-white mb-1">Nenhuma câmera cadastrada</h4>
          <p className="text-xs text-slate-400 max-w-sm mx-auto mb-4">
            Utilize a busca automática de rede para localizar dispositivos na sua rede ou cadastre manualmente.
          </p>
          <div className="flex items-center justify-center gap-3">
            <button
              onClick={onOpenDiscovery}
              className="px-4 py-2 rounded-lg bg-sky-600 hover:bg-sky-500 text-white text-xs font-semibold inline-flex items-center gap-2 transition shadow"
            >
              <Search className="h-4 w-4" />
              <span>Buscar Câmeras na Rede</span>
            </button>
            <button
              onClick={onAddCamera}
              className="px-4 py-2 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs font-semibold inline-flex items-center gap-2 transition border border-slate-700"
            >
              <Plus className="h-4 w-4" />
              <span>Cadastro Manual</span>
            </button>
          </div>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {cameras.map((cam) => {
            const status = streamStatuses[cam.id];
            const isOnline = status?.state === 'online';
            const isConnecting = status?.state === 'connecting';
            const isTestingThis = testingId === cam.id;
            const thisTestResult = testResult?.id === cam.id ? testResult : null;

            return (
              <div
                key={cam.id}
                className="bg-slate-900/90 border border-slate-800/90 rounded-xl p-4 flex flex-col justify-between hover:border-slate-700/80 transition-all shadow-md"
              >
                <div>
                  {/* Top Bar with Name & Status */}
                  <div className="flex items-start justify-between gap-2 mb-3">
                    <div>
                      <h4 className="text-sm font-bold text-white leading-tight">{cam.name}</h4>
                      <p className="text-xs font-mono text-slate-400 mt-0.5">
                        {cam.host}:{cam.rtsp_port}
                      </p>
                    </div>

                    {/* Status Badge */}
                    <div className="flex items-center gap-1.5 shrink-0">
                      {isOnline ? (
                        <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-semibold bg-emerald-500/20 text-emerald-300 border border-emerald-500/30">
                          <span className="h-1.5 w-1.5 rounded-full bg-emerald-400 animate-pulse" />
                          Online
                        </span>
                      ) : isConnecting ? (
                        <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-semibold bg-amber-500/20 text-amber-300 border border-amber-500/30">
                          <Loader2 className="h-2.5 w-2.5 animate-spin" />
                          Conectando
                        </span>
                      ) : (
                        <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-semibold bg-slate-800 text-slate-400 border border-slate-700">
                          <span className="h-1.5 w-1.5 rounded-full bg-slate-500" />
                          Inativo
                        </span>
                      )}
                    </div>
                  </div>

                  {/* RTSP Profile details */}
                  <div className="space-y-1.5 text-xs text-slate-400 bg-slate-950/60 p-2.5 rounded-lg border border-slate-800/60 font-mono mb-3">
                    <div className="truncate text-[11px]" title={cam.rtsp_url}>
                      <span className="text-slate-500">URI: </span>
                      <span className="text-slate-300">{cam.rtsp_url}</span>
                    </div>
                    <div className="flex justify-between text-[11px]">
                      <span>Usuário: <strong className="text-slate-200">{cam.username}</strong></span>
                      <span>Perfil: <strong className="text-sky-400">{cam.stream_profile.toUpperCase()}</strong></span>
                    </div>
                  </div>

                  {/* Telemetry if online */}
                  {status && (
                    <div className="grid grid-cols-2 gap-2 text-[11px] font-mono text-slate-400 mb-3 bg-slate-950/40 p-2 rounded border border-slate-800/40">
                      <div>FPS: <strong className="text-white">{status.fps || 0}</strong></div>
                      <div>Bitrate: <strong className="text-white">{status.bitrate_kbps || 0} kbps</strong></div>
                    </div>
                  )}

                  {/* Test Result Message */}
                  {thisTestResult && (
                    <div
                      className={`text-[11px] p-2 rounded mb-3 border ${
                        thisTestResult.success
                          ? 'bg-emerald-500/10 border-emerald-500/20 text-emerald-300'
                          : 'bg-rose-500/10 border-rose-500/20 text-rose-300'
                      }`}
                    >
                      {thisTestResult.msg}
                    </div>
                  )}
                </div>

                {/* Bottom Actions */}
                <div className="pt-3 border-t border-slate-800/80 flex items-center justify-between gap-2">
                  <div className="flex items-center gap-1.5">
                    {isOnline ? (
                      <button
                        onClick={() => onStopStream(cam.id)}
                        className="px-2.5 py-1.5 rounded-md bg-rose-500/20 hover:bg-rose-500/30 text-rose-300 border border-rose-500/30 text-xs font-semibold flex items-center gap-1.5 transition"
                        title="Interromper Stream"
                      >
                        <Square className="h-3.5 w-3.5 fill-current" />
                        <span>Parar</span>
                      </button>
                    ) : (
                      <button
                        onClick={() => onStartStream(cam.id)}
                        className="px-2.5 py-1.5 rounded-md bg-emerald-500/20 hover:bg-emerald-500/30 text-emerald-300 border border-emerald-500/30 text-xs font-semibold flex items-center gap-1.5 transition"
                        title="Iniciar Stream"
                      >
                        <Play className="h-3.5 w-3.5 fill-current" />
                        <span>Conectar</span>
                      </button>
                    )}

                    <button
                      onClick={() => handleTestExisting(cam.id)}
                      disabled={isTestingThis}
                      className="p-1.5 rounded-md bg-slate-800 hover:bg-slate-700 text-slate-300 hover:text-white border border-slate-700 text-xs transition"
                      title="Testar Conexão RTSP"
                    >
                      {isTestingThis ? (
                        <Loader2 className="h-3.5 w-3.5 animate-spin text-sky-400" />
                      ) : (
                        <Activity className="h-3.5 w-3.5 text-sky-400" />
                      )}
                    </button>
                  </div>

                  <div className="flex items-center gap-1">
                    <button
                      onClick={() => onEditCamera(cam)}
                      className="p-1.5 rounded-md text-slate-400 hover:text-white hover:bg-slate-800 transition"
                      title="Editar"
                    >
                      <Edit2 className="h-3.5 w-3.5" />
                    </button>
                    <button
                      onClick={() => {
                        if (confirm(`Deseja remover a câmera "${cam.name}"?`)) {
                          onDeleteCamera(cam.id);
                        }
                      }}
                      className="p-1.5 rounded-md text-slate-400 hover:text-rose-400 hover:bg-rose-500/10 transition"
                      title="Excluir"
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </button>
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};
