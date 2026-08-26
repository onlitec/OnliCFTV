import React, { useState, useRef } from 'react';
import {
  Camera as CameraIcon,
  RefreshCw,
  Maximize2,
  Minimize2,
  AlertTriangle,
  Loader2,
} from 'lucide-react';
import type { Camera, CameraStreamStatus } from '@/types';

interface VideoCellProps {
  camera?: Camera | null;
  status?: CameraStreamStatus | null;
  onReconnect: (id: string) => void;
  serverPort: number;
}

export const VideoCell: React.FC<VideoCellProps> = ({
  camera,
  status,
  onReconnect,
  serverPort,
}) => {
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [imageError, setImageError] = useState(false);
  const [reloadKey, setReloadKey] = useState(Date.now());
  const cellRef = useRef<HTMLDivElement>(null);

  if (!camera) {
    return (
      <div className="h-full w-full bg-slate-950/80 border border-slate-800/80 rounded-lg flex flex-col items-center justify-center text-slate-600 select-none p-4">
        <CameraIcon className="h-8 w-8 mb-2 opacity-30" />
        <span className="text-xs font-mono">Célula Vazia</span>
      </div>
    );
  }

  const isOnline = status?.state === 'online';
  const isConnecting = status?.state === 'connecting';

  const streamUrl = `http://127.0.0.1:${serverPort}/stream/${camera.id}?t=${reloadKey}`;

  const toggleFullscreen = () => {
    if (!cellRef.current) return;
    if (!document.fullscreenElement) {
      cellRef.current.requestFullscreen().then(() => setIsFullscreen(true)).catch(() => {});
    } else {
      document.exitFullscreen().then(() => setIsFullscreen(false)).catch(() => {});
    }
  };

  const handleManualReconnect = () => {
    setImageError(false);
    setReloadKey(Date.now());
    onReconnect(camera.id);
  };

  return (
    <div
      ref={cellRef}
      className={`relative h-full w-full bg-black border rounded-lg overflow-hidden flex flex-col justify-between group ${
        isOnline
          ? 'border-slate-800 hover:border-sky-500/50'
          : 'border-rose-950/50'
      }`}
    >
      {/* Top Header Overlay */}
      <div className="absolute top-0 inset-x-0 z-20 bg-gradient-to-b from-black/80 via-black/40 to-transparent p-2.5 flex items-center justify-between pointer-events-none">
        <div className="flex items-center gap-2">
          {/* Status Indicator Dot */}
          {isOnline ? (
            <span className="h-2.5 w-2.5 rounded-full bg-emerald-500 shadow-sm shadow-emerald-500/50 animate-pulse" />
          ) : isConnecting ? (
            <span className="h-2.5 w-2.5 rounded-full bg-amber-400 animate-ping" />
          ) : (
            <span className="h-2.5 w-2.5 rounded-full bg-rose-500" />
          )}
          <span className="text-xs font-bold text-white drop-shadow-md tracking-wide">
            {camera.name}
          </span>
        </div>

        {/* Live Metrics */}
        <div className="flex items-center gap-2 text-[11px] font-mono text-slate-300 drop-shadow">
          {isOnline && status && (
            <>
              <span className="bg-black/60 px-1.5 py-0.5 rounded border border-white/10 text-emerald-400 font-semibold">
                {status.fps || 0} FPS
              </span>
              <span className="bg-black/60 px-1.5 py-0.5 rounded border border-white/10 text-slate-300">
                {status.bitrate_kbps || 0} kbps
              </span>
            </>
          )}
          <span className="bg-sky-950/70 text-sky-300 px-1.5 py-0.5 rounded border border-sky-500/30 text-[10px] uppercase font-semibold">
            {camera.stream_profile}
          </span>
        </div>
      </div>

      {/* Video Content Canvas / MJPEG Stream */}
      <div className="relative flex-1 w-full h-full bg-slate-950 flex items-center justify-center overflow-hidden">
        {isOnline && !imageError ? (
          <img
            src={streamUrl}
            alt={camera.name}
            onError={() => setImageError(true)}
            className="w-full h-full object-contain"
          />
        ) : (
          <div className="flex flex-col items-center justify-center p-6 text-center z-10 space-y-3">
            {isConnecting ? (
              <>
                <Loader2 className="h-8 w-8 text-amber-400 animate-spin" />
                <div>
                  <p className="text-sm font-semibold text-amber-300">Conectando ao stream RTSP...</p>
                  <p className="text-xs text-slate-400 mt-0.5 font-mono">{camera.host}:{camera.rtsp_port}</p>
                </div>
              </>
            ) : (
              <>
                <div className="h-10 w-10 rounded-full bg-rose-500/20 border border-rose-500/30 flex items-center justify-center text-rose-400">
                  <AlertTriangle className="h-5 w-5" />
                </div>
                <div>
                  <p className="text-sm font-bold text-rose-300">Câmera offline</p>
                  <p className="text-xs text-slate-400 max-w-xs mt-0.5">
                    {status?.error_message || 'Não foi possível estabelecer fluxo de vídeo com o dispositivo.'}
                  </p>
                </div>
                <button
                  onClick={handleManualReconnect}
                  className="px-3.5 py-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-200 hover:text-white border border-slate-700 text-xs font-semibold flex items-center gap-1.5 transition shadow"
                >
                  <RefreshCw className="h-3.5 w-3.5 text-sky-400" />
                  <span>Reconectar</span>
                </button>
              </>
            )}
          </div>
        )}
      </div>

      {/* Bottom Controls Bar (Visible on Hover) */}
      <div className="absolute bottom-0 inset-x-0 z-20 bg-gradient-to-t from-black/80 via-black/40 to-transparent p-2.5 flex items-center justify-between opacity-0 group-hover:opacity-100 transition-opacity">
        <span className="text-[11px] font-mono text-slate-400 drop-shadow">
          RTSP: {camera.host}
        </span>

        <div className="flex items-center gap-1.5 pointer-events-auto">
          <button
            onClick={handleManualReconnect}
            className="p-1.5 rounded bg-black/60 hover:bg-black/90 text-slate-300 hover:text-white border border-white/10 transition"
            title="Reconectar Stream"
          >
            <RefreshCw className="h-3.5 w-3.5" />
          </button>
          <button
            onClick={toggleFullscreen}
            className="p-1.5 rounded bg-black/60 hover:bg-black/90 text-slate-300 hover:text-white border border-white/10 transition"
            title={isFullscreen ? 'Sair da Tela Cheia' : 'Tela Cheia'}
          >
            {isFullscreen ? <Minimize2 className="h-3.5 w-3.5" /> : <Maximize2 className="h-3.5 w-3.5" />}
          </button>
        </div>
      </div>
    </div>
  );
};
