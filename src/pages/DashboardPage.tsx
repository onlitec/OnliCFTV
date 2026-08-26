import React from 'react';
import {
  Camera,
  Activity,
  CheckCircle2,
  XCircle,
  Radio,
  ArrowRight,
  Play,
} from 'lucide-react';
import type { Camera as CameraType, CameraStreamStatus } from '@/types';

interface DashboardProps {
  cameras: CameraType[];
  streamStatuses: Record<string, CameraStreamStatus>;
  onNavigateTo: (tab: any) => void;
  onStartStream: (id: string) => void;
}

export const DashboardPage: React.FC<DashboardProps> = ({
  cameras,
  streamStatuses,
  onNavigateTo,
  onStartStream,
}) => {
  const totalCameras = cameras.length;
  const onlineCameras = Object.values(streamStatuses).filter((s) => s.state === 'online').length;
  const offlineCameras = totalCameras - onlineCameras;
  const activeStreams = Object.values(streamStatuses).filter(
    (s) => s.state === 'online' || s.state === 'connecting'
  ).length;

  return (
    <div className="p-6 space-y-6 max-h-full overflow-y-auto">
      {/* Welcome Banner */}
      <div className="bg-gradient-to-r from-slate-900 via-sky-950/40 to-slate-900 border border-slate-800 rounded-xl p-6 shadow-lg">
        <div className="flex items-center justify-between">
          <div>
            <span className="text-xs uppercase tracking-wider text-sky-400 font-bold bg-sky-500/10 px-2.5 py-1 rounded-md border border-sky-500/20">
              Painel de Controle VMS
            </span>
            <h2 className="text-2xl font-black text-white mt-2 tracking-tight">
              OnliView Security Station
            </h2>
            <p className="text-xs text-slate-300 mt-1 max-w-xl">
              Plataforma de gerenciamento de vídeo, decodificação em tempo real e monitoramento profissional de CFTV IP.
            </p>
          </div>
          <button
            onClick={() => onNavigateTo('live')}
            className="px-5 py-2.5 rounded-lg bg-sky-600 hover:bg-sky-500 text-white text-sm font-semibold shadow-lg shadow-sky-950 flex items-center gap-2 transition shrink-0"
          >
            <Radio className="h-4 w-4" />
            <span>Abrir Visualização</span>
            <ArrowRight className="h-4 w-4" />
          </button>
        </div>
      </div>

      {/* Metrics Row */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <div className="bg-slate-900/90 border border-slate-800 rounded-xl p-4 shadow">
          <div className="flex items-center justify-between text-slate-400 mb-2">
            <span className="text-xs font-semibold">Total de Câmeras</span>
            <Camera className="h-4 w-4 text-sky-400" />
          </div>
          <div className="text-2xl font-bold text-white font-mono">{totalCameras}</div>
          <div className="text-[11px] text-slate-400 mt-1">Dispositivos cadastrados</div>
        </div>

        <div className="bg-slate-900/90 border border-slate-800 rounded-xl p-4 shadow">
          <div className="flex items-center justify-between text-slate-400 mb-2">
            <span className="text-xs font-semibold">Câmeras Online</span>
            <CheckCircle2 className="h-4 w-4 text-emerald-400" />
          </div>
          <div className="text-2xl font-bold text-emerald-400 font-mono">{onlineCameras}</div>
          <div className="text-[11px] text-slate-400 mt-1">Transmitindo normalmente</div>
        </div>

        <div className="bg-slate-900/90 border border-slate-800 rounded-xl p-4 shadow">
          <div className="flex items-center justify-between text-slate-400 mb-2">
            <span className="text-xs font-semibold">Câmeras Offline</span>
            <XCircle className="h-4 w-4 text-rose-400" />
          </div>
          <div className="text-2xl font-bold text-rose-400 font-mono">{offlineCameras}</div>
          <div className="text-[11px] text-slate-400 mt-1">Inativas ou desconectadas</div>
        </div>

        <div className="bg-slate-900/90 border border-slate-800 rounded-xl p-4 shadow">
          <div className="flex items-center justify-between text-slate-400 mb-2">
            <span className="text-xs font-semibold">Motor de Vídeo</span>
            <Activity className="h-4 w-4 text-sky-400" />
          </div>
          <div className="text-2xl font-bold text-white font-mono">{activeStreams} Ativos</div>
          <div className="text-[11px] text-slate-400 mt-1">Sessões FFmpeg em execução</div>
        </div>
      </div>

      {/* Quick Camera Grid Overview */}
      <div className="bg-slate-900/90 border border-slate-800 rounded-xl p-5 shadow">
        <div className="flex items-center justify-between mb-4">
          <div>
            <h3 className="text-base font-bold text-white">Status dos Dispositivos</h3>
            <p className="text-xs text-slate-400">Visão rápida de conectividade e telemetria</p>
          </div>
          <button
            onClick={() => onNavigateTo('cameras')}
            className="text-xs text-sky-400 hover:text-sky-300 font-semibold flex items-center gap-1"
          >
            <span>Gerenciar Câmeras</span>
            <ArrowRight className="h-3 w-3" />
          </button>
        </div>

        {cameras.length === 0 ? (
          <div className="py-8 text-center text-slate-500 text-xs">
            Nenhuma câmera configurada. Clique em "Câmeras" para adicionar seu primeiro dispositivo.
          </div>
        ) : (
          <div className="divide-y divide-slate-800/80">
            {cameras.map((cam) => {
              const st = streamStatuses[cam.id];
              const isOnline = st?.state === 'online';
              const isConnecting = st?.state === 'connecting';

              return (
                <div key={cam.id} className="py-3 flex items-center justify-between">
                  <div className="flex items-center gap-3">
                    <div
                      className={`h-2.5 w-2.5 rounded-full ${
                        isOnline
                          ? 'bg-emerald-400 shadow-sm shadow-emerald-500/50'
                          : isConnecting
                          ? 'bg-amber-400 animate-ping'
                          : 'bg-rose-500'
                      }`}
                    />
                    <div>
                      <h4 className="text-sm font-semibold text-white">{cam.name}</h4>
                      <p className="text-xs text-slate-400 font-mono">
                        {cam.host}:{cam.rtsp_port} • Perfil: {cam.stream_profile.toUpperCase()}
                      </p>
                    </div>
                  </div>

                  <div className="flex items-center gap-4 text-xs font-mono text-slate-400">
                    {isOnline && st && (
                      <div className="hidden sm:flex items-center gap-3 text-slate-300">
                        <span>FPS: <strong className="text-emerald-400">{st.fps}</strong></span>
                        <span>Bitrate: <strong>{st.bitrate_kbps} kbps</strong></span>
                      </div>
                    )}

                    {!isOnline && (
                      <button
                        onClick={() => onStartStream(cam.id)}
                        className="px-3 py-1 rounded bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs font-semibold flex items-center gap-1.5 transition border border-slate-700"
                      >
                        <Play className="h-3 w-3 text-emerald-400 fill-current" />
                        <span>Conectar</span>
                      </button>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
};
