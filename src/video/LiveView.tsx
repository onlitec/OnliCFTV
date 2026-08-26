import React, { useState, useEffect } from 'react';
import {
  Grid2X2,
  Grid3X3,
  Square,
  Play,
  Radio,
  ArrowLeft,
  Info,
} from 'lucide-react';
import type { Camera, CameraStreamStatus } from '@/types';
import { VideoCell } from '@/video/VideoCell';

interface LiveViewProps {
  cameras: Camera[];
  streamStatuses: Record<string, CameraStreamStatus>;
  onReconnect: (id: string) => void;
  onStartAll: () => void;
  onStopAll: () => void;
  serverPort: number;
  selectedCameraId?: string | null;
  onBackToDashboard?: () => void;
}

export const LiveView: React.FC<LiveViewProps> = ({
  cameras,
  streamStatuses,
  onReconnect,
  onStartAll,
  onStopAll,
  serverPort,
  selectedCameraId,
  onBackToDashboard,
}) => {
  const [layout, setLayout] = useState<1 | 4 | 9>(selectedCameraId ? 1 : 4);
  const [activeCamId, setActiveCamId] = useState<string | null>(selectedCameraId || null);

  useEffect(() => {
    if (selectedCameraId) {
      setActiveCamId(selectedCameraId);
      setLayout(1);
    }
  }, [selectedCameraId]);

  const focusedCamera = cameras.find((c) => c.id === activeCamId);
  const focusedStatus = activeCamId ? streamStatuses[activeCamId] : null;

  // Auto-connect focused camera if not online
  useEffect(() => {
    if (activeCamId && !streamStatuses[activeCamId]) {
      onReconnect(activeCamId);
    }
  }, [activeCamId, onReconnect, streamStatuses]);

  return (
    <div className="h-full flex flex-col bg-slate-950 p-4 space-y-3 select-none">
      {/* Top Toolbar */}
      <div className="flex items-center justify-between bg-slate-900/90 border border-slate-800/80 px-4 py-2.5 rounded-xl shadow-lg shrink-0">
        <div className="flex items-center gap-3">
          {onBackToDashboard && (
            <button
              onClick={onBackToDashboard}
              className="px-3 py-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 text-sky-400 hover:text-sky-300 text-xs font-bold flex items-center gap-1.5 transition border border-slate-700 shadow"
            >
              <ArrowLeft className="h-4 w-4" />
              <span>Voltar ao Painel</span>
            </button>
          )}

          <div className="flex items-center gap-2">
            <Radio className="h-4 w-4 text-emerald-400 animate-pulse" />
            <h3 className="text-sm font-bold text-white tracking-tight">
              {layout === 1 && focusedCamera ? (
                <span>
                  Visualização Rápida: <strong className="text-sky-400">{focusedCamera.name}</strong> ({focusedCamera.host})
                </span>
              ) : (
                <span>Mosaico Geral de Monitoramento</span>
              )}
            </h3>
          </div>
        </div>

        {/* Action Controls & Layout Switcher */}
        <div className="flex items-center gap-2">
          <div className="flex items-center bg-slate-950 p-1 rounded-lg border border-slate-800">
            <button
              onClick={() => {
                setLayout(1);
                if (!activeCamId && cameras.length > 0) {
                  setActiveCamId(cameras[0].id);
                }
              }}
              className={`p-1.5 rounded-md text-xs font-semibold flex items-center gap-1 transition ${
                layout === 1
                  ? 'bg-sky-600 text-white shadow'
                  : 'text-slate-400 hover:text-white'
              }`}
              title="1 Câmera em Destaque (1x1)"
            >
              <Square className="h-3.5 w-3.5" />
              <span>1x1</span>
            </button>
            <button
              onClick={() => setLayout(4)}
              className={`p-1.5 rounded-md text-xs font-semibold flex items-center gap-1 transition ${
                layout === 4
                  ? 'bg-sky-600 text-white shadow'
                  : 'text-slate-400 hover:text-white'
              }`}
              title="Mosaico 4 Câmeras (2x2)"
            >
              <Grid2X2 className="h-3.5 w-3.5" />
              <span>2x2</span>
            </button>
            <button
              onClick={() => setLayout(9)}
              className={`p-1.5 rounded-md text-xs font-semibold flex items-center gap-1 transition ${
                layout === 9
                  ? 'bg-sky-600 text-white shadow'
                  : 'text-slate-400 hover:text-white'
              }`}
              title="Mosaico 9 Câmeras (3x3)"
            >
              <Grid3X3 className="h-3.5 w-3.5" />
              <span>3x3</span>
            </button>
          </div>

          <button
            onClick={onStartAll}
            className="px-3 py-1.5 rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-bold flex items-center gap-1.5 transition shadow"
          >
            <Play className="h-3.5 w-3.5 fill-current" />
            <span>Conectar Todas</span>
          </button>
          <button
            onClick={onStopAll}
            className="px-3 py-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-300 text-xs font-medium border border-slate-700 transition"
          >
            Parar Todas
          </button>
        </div>
      </div>

      {/* Technician OSD / Focus Helper Banner when 1 camera is selected */}
      {layout === 1 && focusedCamera && (
        <div className="bg-sky-950/40 border border-sky-500/30 px-4 py-2 rounded-lg flex items-center justify-between text-xs text-slate-300 shrink-0">
          <div className="flex items-center gap-2">
            <Info className="h-4 w-4 text-sky-400 shrink-0" />
            <span>
              <strong>Dica de Instalação:</strong> Ajuste o foco da lente e verifique o OSD da câmera (Nome: <em>{focusedCamera.name}</em>, IP: <code>{focusedCamera.host}</code>).
            </span>
          </div>
          {focusedStatus && (
            <div className="flex items-center gap-3 font-mono text-[11px] text-emerald-400">
              <span>FPS: <strong>{focusedStatus.fps}</strong></span>
              <span>Bitrate: <strong>{focusedStatus.bitrate_kbps} kbps</strong></span>
            </div>
          )}
        </div>
      )}

      {/* Video Grid Canvas */}
      <div className="flex-1 min-h-0 bg-slate-950 rounded-xl overflow-hidden border border-slate-800/80">
        {cameras.length === 0 ? (
          <div className="h-full flex flex-col items-center justify-center text-center p-6 space-y-3">
            <Radio className="h-12 w-12 text-slate-700" />
            <h4 className="text-base font-bold text-white">Nenhuma câmera disponível no mosaico</h4>
            <p className="text-xs text-slate-500 max-w-sm">
              Cadastre câmeras na Dashboard através da busca inteligente para visualizá-las aqui.
            </p>
          </div>
        ) : layout === 1 && focusedCamera ? (
          <div className="h-full w-full p-2">
            <VideoCell
              camera={focusedCamera}
              status={focusedStatus}
              onReconnect={onReconnect}
              serverPort={serverPort}
            />
          </div>
        ) : (
          <div
            className={`h-full w-full p-2 grid gap-2 ${
              layout === 4 ? 'grid-cols-2 grid-rows-2' : 'grid-cols-3 grid-rows-3'
            }`}
          >
            {cameras.slice(0, layout).map((cam) => (
              <div
                key={cam.id}
                onClick={() => {
                  setActiveCamId(cam.id);
                  setLayout(1);
                }}
                className="h-full w-full cursor-pointer group"
                title="Clique duplo ou clique para expandir esta câmera"
              >
                <VideoCell
                  camera={cam}
                  status={streamStatuses[cam.id]}
                  onReconnect={onReconnect}
                  serverPort={serverPort}
                />
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};
