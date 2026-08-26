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
  Eye,
  Camera as CameraIcon,
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

  return (
    <div className="p-6 space-y-6 max-h-full overflow-y-auto select-none">
      {/* Top Bar */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-black text-white tracking-tight flex items-center gap-2">
            <span>Painel de Instalação e Câmeras</span>
            <span className="text-xs font-bold px-2 py-0.5 rounded-full bg-emerald-500/20 text-emerald-300 border border-emerald-500/30">
              {cameras.length} Cadastrada(s)
            </span>
          </h2>
          <p className="text-xs text-slate-400 mt-0.5">
            Clique em qualquer câmera abaixo para abrir imediatamente a imagem e ajustar enquadramento, foco e OSD.
          </p>
        </div>

        <div className="flex items-center gap-2.5">
          <button
            onClick={() => onOpenLiveCamera('')}
            className="px-3.5 py-2 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs font-semibold border border-slate-700 flex items-center gap-1.5 transition"
          >
            <Radio className="h-4 w-4 text-sky-400" />
            <span>Ver Mosaico Geral</span>
          </button>
          <button
            onClick={onAddCamera}
            className="px-4 py-2 rounded-lg bg-sky-600 hover:bg-sky-500 text-white text-xs font-bold shadow-md shadow-sky-950 flex items-center gap-1.5 transition"
          >
            <Plus className="h-4 w-4" />
            <span>Cadastrar Manual</span>
          </button>
        </div>
      </div>

      {/* CENTER: Added Cameras Grid */}
      <div>
        {cameras.length === 0 ? (
          <div className="bg-slate-900/60 border border-dashed border-slate-800 rounded-xl p-8 text-center">
            <CameraIcon className="h-10 w-10 text-slate-600 mx-auto mb-2" />
            <h4 className="text-sm font-bold text-white mb-1">Nenhuma câmera cadastrada ainda</h4>
            <p className="text-xs text-slate-400 max-w-sm mx-auto mb-3">
              Utilize o painel de busca inteligente abaixo para localizar e adicionar dispositivos da rede local com 1 clique.
            </p>
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
                  onClick={() => onOpenLiveCamera(cam.id)}
                  className="group bg-slate-900/90 hover:bg-slate-900 border border-slate-800 hover:border-sky-500/60 rounded-xl p-4 flex flex-col justify-between transition-all shadow-lg hover:shadow-sky-950/40 cursor-pointer relative overflow-hidden"
                >
                  {/* Top indicator bar */}
                  <div className="flex items-start justify-between gap-2 mb-3">
                    <div className="flex items-center gap-2.5">
                      <div className="h-9 w-9 rounded-lg bg-slate-800 group-hover:bg-sky-500/20 group-hover:text-sky-400 flex items-center justify-center text-slate-400 transition">
                        <CameraIcon className="h-5 w-5" />
                      </div>
                      <div>
                        <h4 className="text-sm font-bold text-white group-hover:text-sky-300 transition leading-tight">
                          {cam.name}
                        </h4>
                        <p className="text-xs font-mono text-slate-400 mt-0.5">
                          {cam.host}:{cam.rtsp_port}
                        </p>
                      </div>
                    </div>

                    {/* Status Badge */}
                    <div className="shrink-0">
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
                          Inativa
                        </span>
                      )}
                    </div>
                  </div>

                  {/* RTSP & Telemetry Box */}
                  <div className="space-y-1.5 text-xs text-slate-400 bg-slate-950/70 p-2.5 rounded-lg border border-slate-800/80 font-mono mb-3">
                    <div className="truncate text-[11px]" title={cam.rtsp_url}>
                      <span className="text-slate-500">URI: </span>
                      <span className="text-slate-300">{cam.rtsp_url}</span>
                    </div>
                    <div className="flex justify-between text-[11px]">
                      <span>Usuário: <strong className="text-slate-200">{cam.username}</strong></span>
                      <span>Perfil: <strong className="text-sky-400">{cam.stream_profile.toUpperCase()}</strong></span>
                    </div>
                    {status && isOnline && (
                      <div className="flex justify-between pt-1 border-t border-slate-800/80 text-[11px] text-emerald-400">
                        <span>FPS: <strong>{status.fps}</strong></span>
                        <span>Bitrate: <strong>{status.bitrate_kbps} kbps</strong></span>
                      </div>
                    )}
                  </div>

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

                  {/* Direct Live View Action Button */}
                  <div className="mb-3">
                    <div className="w-full py-2 rounded-lg bg-sky-600/20 group-hover:bg-sky-600 text-sky-300 group-hover:text-white border border-sky-500/30 text-xs font-bold flex items-center justify-center gap-2 transition shadow">
                      <Eye className="h-4 w-4" />
                      <span>Abrir Imagem Ao Vivo & Ajustar OSD</span>
                    </div>
                  </div>

                  {/* Bottom Controls */}
                  <div
                    className="pt-2 border-t border-slate-800/80 flex items-center justify-between"
                    onClick={(e) => e.stopPropagation()}
                  >
                    <div className="flex items-center gap-1.5">
                      {isOnline ? (
                        <button
                          onClick={() => onStopStream(cam.id)}
                          className="px-2 py-1 rounded bg-rose-500/20 hover:bg-rose-500/30 text-rose-300 border border-rose-500/30 text-xs font-semibold flex items-center gap-1 transition"
                          title="Interromper Stream"
                        >
                          <Square className="h-3 w-3 fill-current" />
                          <span>Parar</span>
                        </button>
                      ) : (
                        <button
                          onClick={() => onStartStream(cam.id)}
                          className="px-2 py-1 rounded bg-emerald-500/20 hover:bg-emerald-500/30 text-emerald-300 border border-emerald-500/30 text-xs font-semibold flex items-center gap-1 transition"
                          title="Conectar Stream"
                        >
                          <Play className="h-3 w-3 fill-current" />
                          <span>Conectar</span>
                        </button>
                      )}

                      <button
                        onClick={(e) => handleTestExisting(e, cam.id)}
                        disabled={isTestingThis}
                        className="p-1 rounded bg-slate-800 hover:bg-slate-700 text-slate-300 border border-slate-700 text-xs transition"
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
                        className="p-1 rounded text-slate-400 hover:text-white hover:bg-slate-800 transition"
                        title="Editar Câmera"
                      >
                        <Edit2 className="h-3.5 w-3.5" />
                      </button>
                      <button
                        onClick={() => {
                          if (confirm(`Deseja excluir a câmera "${cam.name}"?`)) {
                            onDeleteCamera(cam.id);
                          }
                        }}
                        className="p-1 rounded text-slate-400 hover:text-rose-400 hover:bg-rose-500/10 transition"
                        title="Excluir Câmera"
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

      {/* BOTTOM: Integrated Intelligent Discovery Panel */}
      <div>
        <DiscoveryPanel
          discoveredDevices={discoveredDevices}
          isScanning={isScanning}
          onRefreshScan={onRefreshScan}
          onAdded={onDataChanged}
          onAddSingle={onAddSingleFromDiscovery}
        />
      </div>
    </div>
  );
};
