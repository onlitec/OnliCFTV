import React, { useState, useEffect } from 'react';
import {
  Camera,
  Loader2,
  AlertTriangle,
  Maximize2,
  Lock,
} from 'lucide-react';
import type { DiscoveredDevice, QuickViewConnectInput } from '@/types';
import { api } from '@/services/api';

interface DeviceThumbnailCellProps {
  device: DiscoveredDevice;
  isAutoPreviewEnabled?: boolean;
  onOpenQuickView: (device: DiscoveredDevice) => void;
}

export const DeviceThumbnailCell: React.FC<DeviceThumbnailCellProps> = ({
  device,
  isAutoPreviewEnabled = false,
  onOpenQuickView,
}) => {
  const isVideoDevice = [
    'ip_camera',
    'intercom',
    'nvr',
    'dvr',
    'ptz',
    'traffic_lpr',
    'thermal',
  ].includes(device.device_type);

  if (!isVideoDevice) {
    return (
      <div className="w-28 h-16 rounded-lg bg-slate-100 dark:bg-slate-950/40 border border-slate-200 dark:border-slate-800/40 flex items-center justify-center text-slate-400 dark:text-slate-600 text-[10px] italic">
        Sem vídeo
      </div>
    );
  }

  const [previewState, setPreviewState] = useState<'idle' | 'prompt' | 'connecting' | 'live' | 'error'>('idle');
  const [streamUrl, setStreamUrl] = useState<string | null>(null);
  const [password, setPassword] = useState('');
  const [username] = useState('admin');
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [reloadKey, setReloadKey] = useState(Date.now());

  useEffect(() => {
    if (isAutoPreviewEnabled && previewState === 'idle') {
      handleStartStream('admin', 'Onlitec@2026');
    }
  }, [isAutoPreviewEnabled]);

  const handleStartStream = async (user: string, pass: string) => {
    setPreviewState('connecting');
    setErrorMsg(null);

    try {
      const input: QuickViewConnectInput = {
        ip: device.ip,
        rtsp_port: device.rtsp_port || 554,
        http_port: device.http_port || 80,
        username: user || 'admin',
        password: pass || undefined,
      };

      const url = await api.startDevicePreview(input);
      setStreamUrl(url);
      setReloadKey(Date.now());
      setPreviewState('live');
    } catch (err: any) {
      setErrorMsg(err?.toString() || 'Erro ao conectar preview');
      setPreviewState('error');
    }
  };

  const handlePromptSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    handleStartStream(username, password);
  };

  return (
    <div className="relative w-28 h-16 rounded-lg overflow-hidden border border-slate-200 dark:border-slate-800 bg-slate-100 dark:bg-slate-950 shadow-sm group shrink-0 select-none">
      {/* 1. IDLE STATE */}
      {previewState === 'idle' && (
        <button
          type="button"
          onClick={() => setPreviewState('prompt')}
          className="w-full h-full flex flex-col items-center justify-center gap-1 bg-slate-50 dark:bg-slate-900/80 hover:bg-slate-100 dark:hover:bg-slate-800/90 text-slate-600 dark:text-slate-400 hover:text-sky-600 dark:hover:text-sky-300 transition"
          title="Clique para iniciar preview ao vivo"
        >
          <Camera className="h-4 w-4 text-sky-600 dark:text-sky-400" />
          <span className="text-[10px] font-bold font-sans">Preview</span>
        </button>
      )}

      {/* 2. PASSWORD PROMPT IN-CELL */}
      {previewState === 'prompt' && (
        <form
          onSubmit={handlePromptSubmit}
          className="w-full h-full p-1 bg-white dark:bg-slate-900 flex flex-col justify-between"
        >
          <div className="flex items-center gap-1 text-[9px] text-slate-600 dark:text-slate-300 font-mono">
            <Lock className="h-2.5 w-2.5 text-sky-600 dark:text-sky-400" />
            <span>Senha:</span>
          </div>
          <input
            type="password"
            autoFocus
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            placeholder="Senha"
            className="w-full bg-slate-50 dark:bg-slate-950 border border-slate-300 dark:border-slate-700 rounded px-1.5 py-0.5 text-[10px] text-slate-900 dark:text-white focus:outline-none focus:border-sky-500 font-mono"
          />
          <div className="flex items-center justify-between gap-1 pt-0.5">
            <button
              type="button"
              onClick={() => setPreviewState('idle')}
              className="text-[9px] text-slate-500 dark:text-slate-400 hover:text-slate-800 dark:hover:text-slate-200"
            >
              ✕
            </button>
            <button
              type="submit"
              className="px-1.5 py-0.5 bg-sky-600 hover:bg-sky-500 text-white text-[9px] font-bold rounded"
            >
              OK
            </button>
          </div>
        </form>
      )}

      {/* 3. CONNECTING STATE */}
      {previewState === 'connecting' && (
        <div className="w-full h-full flex flex-col items-center justify-center gap-1 bg-slate-50 dark:bg-slate-950 text-slate-600 dark:text-slate-400">
          <Loader2 className="h-4 w-4 animate-spin text-sky-600 dark:text-sky-400" />
          <span className="text-[9px] font-mono">Conectando...</span>
        </div>
      )}

      {/* 4. LIVE STREAM STATE */}
      {previewState === 'live' && streamUrl && (
        <div
          onClick={() => onOpenQuickView(device)}
          className="relative w-full h-full cursor-pointer overflow-hidden"
          title="Clique para abrir a visualização completa e alterar OSD/Nome"
        >
          <img
            src={`${streamUrl}?t=${reloadKey}`}
            alt={device.name}
            onError={() => setPreviewState('error')}
            className="w-full h-full object-cover group-hover:scale-105 transition-transform duration-200"
          />

          {/* Live Badge */}
          <div className="absolute top-1 left-1 bg-black/70 backdrop-blur-sm px-1.5 py-0.2 rounded flex items-center gap-1 text-[8px] font-bold text-emerald-400 border border-emerald-500/30 pointer-events-none">
            <span className="h-1.5 w-1.5 rounded-full bg-emerald-500 animate-pulse" />
            <span>LIVE</span>
          </div>

          {/* Hover Zoom Icon Overlay */}
          <div className="absolute inset-0 bg-sky-950/40 opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center gap-1 text-white text-[10px] font-bold">
            <Maximize2 className="h-3.5 w-3.5" />
            <span>Expandir</span>
          </div>
        </div>
      )}

      {/* 5. ERROR STATE */}
      {previewState === 'error' && (
        <div className="w-full h-full p-1 bg-rose-50 dark:bg-rose-950/40 border border-rose-200 dark:border-rose-900/50 flex flex-col items-center justify-center text-center">
          <AlertTriangle className="h-3 w-3 text-rose-500 dark:text-rose-400" />
          <span className="text-[8px] text-rose-600 dark:text-rose-300 font-mono mt-0.5 truncate max-w-full px-1" title={errorMsg || ''}>
            {errorMsg ? 'Erro Conexão' : 'Sem Imagem'}
          </span>
          <button
            onClick={() => setPreviewState('prompt')}
            className="mt-0.5 text-[8px] text-sky-600 dark:text-sky-400 hover:underline"
          >
            Tentar de novo
          </button>
        </div>
      )}
    </div>
  );
};
