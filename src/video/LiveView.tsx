import React, { useState } from 'react';
import { Square, Radio, Play } from 'lucide-react';
import type { Camera, CameraStreamStatus } from '@/types';
import { VideoCell } from './VideoCell';

interface LiveViewProps {
  cameras: Camera[];
  streamStatuses: Record<string, CameraStreamStatus>;
  onReconnect: (id: string) => void;
  onStartAll: () => void;
  onStopAll: () => void;
  serverPort: number;
}

export const LiveView: React.FC<LiveViewProps> = ({
  cameras,
  streamStatuses,
  onReconnect,
  onStartAll,
  onStopAll,
  serverPort,
}) => {
  const [gridLayout, setGridLayout] = useState<1 | 4 | 9>(4);

  // Filter only enabled cameras
  const enabledCameras = cameras.filter((c) => c.enabled);

  // Generate grid slots based on layout
  const slotsCount = gridLayout;
  const slots: (Camera | null)[] = [];
  for (let i = 0; i < slotsCount; i++) {
    slots.push(enabledCameras[i] || null);
  }

  const gridClass =
    gridLayout === 1
      ? 'grid-cols-1 grid-rows-1'
      : gridLayout === 4
      ? 'grid-cols-2 grid-rows-2'
      : 'grid-cols-3 grid-rows-3';

  return (
    <div className="h-full flex flex-col bg-slate-950 select-none">
      {/* Control Toolbar */}
      <div className="h-12 bg-slate-900/90 border-b border-slate-800 px-4 flex items-center justify-between shrink-0">
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-1.5 text-xs text-slate-300 font-semibold">
            <Radio className="h-4 w-4 text-sky-400" />
            <span>Mosaico Ao Vivo</span>
          </div>

          <div className="h-4 w-[1px] bg-slate-800" />

          {/* Quick Stream Controls */}
          <button
            onClick={onStartAll}
            className="px-2.5 py-1 rounded bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs font-medium border border-slate-700 flex items-center gap-1.5 transition"
            title="Conectar todas as câmeras"
          >
            <Play className="h-3 w-3 text-emerald-400 fill-current" />
            <span>Conectar Todas</span>
          </button>

          <button
            onClick={onStopAll}
            className="px-2.5 py-1 rounded bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs font-medium border border-slate-700 flex items-center gap-1.5 transition"
            title="Parar todos os streams"
          >
            <Square className="h-3 w-3 text-rose-400 fill-current" />
            <span>Parar Todas</span>
          </button>
        </div>

        {/* Layout Switchers */}
        <div className="flex items-center gap-1.5">
          <span className="text-[11px] text-slate-400 mr-1">Layout:</span>
          
          <button
            onClick={() => setGridLayout(1)}
            className={`p-1.5 rounded border text-xs font-mono font-bold transition flex items-center gap-1 ${
              gridLayout === 1
                ? 'bg-sky-500/20 border-sky-500/50 text-sky-300'
                : 'bg-slate-950 border-slate-800 text-slate-400 hover:border-slate-700'
            }`}
            title="1 Câmera (1x1)"
          >
            1x1
          </button>

          <button
            onClick={() => setGridLayout(4)}
            className={`p-1.5 rounded border text-xs font-mono font-bold transition flex items-center gap-1 ${
              gridLayout === 4
                ? 'bg-sky-500/20 border-sky-500/50 text-sky-300'
                : 'bg-slate-950 border-slate-800 text-slate-400 hover:border-slate-700'
            }`}
            title="4 Câmeras (2x2)"
          >
            2x2
          </button>

          <button
            onClick={() => setGridLayout(9)}
            className={`p-1.5 rounded border text-xs font-mono font-bold transition flex items-center gap-1 ${
              gridLayout === 9
                ? 'bg-sky-500/20 border-sky-500/50 text-sky-300'
                : 'bg-slate-950 border-slate-800 text-slate-400 hover:border-slate-700'
            }`}
            title="9 Câmeras (3x3)"
          >
            3x3
          </button>
        </div>
      </div>

      {/* Grid Canvas */}
      <div className="flex-1 p-2.5 overflow-hidden">
        {enabledCameras.length === 0 ? (
          <div className="h-full flex flex-col items-center justify-center text-center p-8 bg-slate-900/40 rounded-xl border border-dashed border-slate-800">
            <Radio className="h-10 w-10 text-slate-600 mb-3" />
            <h4 className="text-base font-bold text-white mb-1">Nenhuma câmera disponível no mosaico</h4>
            <p className="text-xs text-slate-400 max-w-sm">
              Cadastre e ative suas câmeras no menu "Câmeras" para visualizá-las ao vivo.
            </p>
          </div>
        ) : (
          <div className={`grid ${gridClass} gap-2.5 h-full w-full`}>
            {slots.map((cam, idx) => (
              <VideoCell
                key={cam ? cam.id : `empty-slot-${idx}`}
                camera={cam}
                status={cam ? streamStatuses[cam.id] : null}
                onReconnect={onReconnect}
                serverPort={serverPort}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
};
