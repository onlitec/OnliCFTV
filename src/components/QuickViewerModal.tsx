import React, { useState, useEffect, useRef } from 'react';
import {
  X,
  Eye,
  EyeOff,
  Radio,
  Lock,
  Camera as CameraIcon,
  RefreshCw,
  Maximize2,
  Minimize2,
  Loader2,
  Sliders,
  Check,
  Save,
  AlertTriangle,
  Tv,
  Activity,
  PlusCircle,
} from 'lucide-react';
import type {
  DiscoveredDevice,
  QuickViewConnectInput,
  QuickViewSessionInfo,
  QuickViewSetDeviceNameInput,
  QuickViewSetOsdInput,
  CreateCameraInput,
} from '@/types';
import { api } from '@/services/api';

interface QuickViewerModalProps {
  device: DiscoveredDevice | null;
  isOpen: boolean;
  onClose: () => void;
  onDeviceUpdated?: () => void;
  onAddAsCamera?: (prefill: CreateCameraInput) => void;
}

export const QuickViewerModal: React.FC<QuickViewerModalProps> = ({
  device,
  isOpen,
  onClose,
  onDeviceUpdated,
  onAddAsCamera,
}) => {
  if (!isOpen || !device) return null;

  // Step: 'auth' -> 'view'
  const [step, setStep] = useState<'auth' | 'view'>('auth');
  const [username, setUsername] = useState('admin');
  const [password, setPassword] = useState('');
  const [showPassword, setShowPassword] = useState(false);
  const [isConnecting, setIsConnecting] = useState(false);
  const [connectError, setConnectError] = useState<string | null>(null);

  // Session info
  const [session, setSession] = useState<QuickViewSessionInfo | null>(null);
  const [imageError, setImageError] = useState(false);
  const [reloadKey, setReloadKey] = useState(Date.now());
  const [isFullscreen, setIsFullscreen] = useState(false);
  const videoContainerRef = useRef<HTMLDivElement>(null);

  // Editable fields
  const [editDeviceName, setEditDeviceName] = useState('');
  const [isSavingName, setIsSavingName] = useState(false);
  const [nameSaveMsg, setNameSaveMsg] = useState<{ success: boolean; text: string } | null>(null);

  const [editOsd, setEditOsd] = useState('');
  const [isSavingOsd, setIsSavingOsd] = useState(false);
  const [osdSaveMsg, setOsdSaveMsg] = useState<{ success: boolean; text: string } | null>(null);

  // Auto-fill credentials previously used successfully on this device, so the technician
  // doesn't have to retype the password every time they reopen Quick View for the same IP.
  useEffect(() => {
    setStep('auth');
    setConnectError(null);
    setSession(null);
    setImageError(false);
    setNameSaveMsg(null);
    setOsdSaveMsg(null);
    setPassword('');

    api.getDeviceCredentials(device.ip)
      .then((cached) => {
        if (cached) {
          setUsername(cached.username);
          setPassword(cached.password);
        }
      })
      .catch(console.error);
  }, [device]);

  // Handle Disconnect on modal close
  useEffect(() => {
    return () => {
      if (device) {
        api.quickViewDisconnect(device.ip).catch(console.error);
      }
    };
  }, [device]);

  const handleConnect = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsConnecting(true);
    setConnectError(null);

    try {
      const input: QuickViewConnectInput = {
        ip: device.ip,
        rtsp_port: device.rtsp_port || 554,
        http_port: device.http_port || 80,
        username: username.trim() || 'admin',
        password: password || undefined,
      };

      const res = await api.quickViewConnect(input);
      setSession(res);
      setEditDeviceName(res.device_name || device.name);
      setEditOsd(res.osd_text || '');
      setStep('view');
      setReloadKey(Date.now());
    } catch (err: any) {
      setConnectError(err?.toString() || 'Falha ao autenticar ou conectar no fluxo de vídeo');
    } finally {
      setIsConnecting(false);
    }
  };

  const handleReconnectLive = () => {
    setImageError(false);
    setReloadKey(Date.now());
  };

  const handleSaveDeviceName = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!session || !editDeviceName.trim()) return;

    setIsSavingName(true);
    setNameSaveMsg(null);

    try {
      const input: QuickViewSetDeviceNameInput = {
        ip: session.ip,
        http_port: session.http_port,
        username,
        password: password || undefined,
        new_name: editDeviceName.trim(),
      };
      await api.quickViewSetDeviceName(input);
      setNameSaveMsg({ success: true, text: 'Device Name alterado com sucesso no dispositivo!' });
      setSession((prev) => (prev ? { ...prev, device_name: editDeviceName.trim() } : null));
      if (onDeviceUpdated) onDeviceUpdated();
    } catch (err: any) {
      setNameSaveMsg({ success: false, text: err?.toString() || 'Erro ao alterar Device Name' });
    } finally {
      setIsSavingName(false);
    }
  };

  const handleSaveOsd = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!session) return;

    setIsSavingOsd(true);
    setOsdSaveMsg(null);

    try {
      const input: QuickViewSetOsdInput = {
        ip: session.ip,
        http_port: session.http_port,
        channel_id: 1,
        username,
        password: password || undefined,
        new_osd: editOsd.trim(),
      };
      await api.quickViewSetOsd(input);
      setOsdSaveMsg({ success: true, text: 'OSD gravado com sucesso na imagem do dispositivo!' });
      setSession((prev) => (prev ? { ...prev, osd_text: editOsd.trim() } : null));
      if (onDeviceUpdated) onDeviceUpdated();
    } catch (err: any) {
      setOsdSaveMsg({ success: false, text: err?.toString() || 'Erro ao gravar OSD' });
    } finally {
      setIsSavingOsd(false);
    }
  };

  const toggleFullscreen = () => {
    if (!videoContainerRef.current) return;
    if (!document.fullscreenElement) {
      videoContainerRef.current.requestFullscreen().then(() => setIsFullscreen(true)).catch(console.error);
    } else {
      document.exitFullscreen().then(() => setIsFullscreen(false)).catch(console.error);
    }
  };

  const handleSnapshot = () => {
    if (!session?.local_mjpeg_url) return;
    const link = document.createElement('a');
    link.href = session.local_mjpeg_url;
    link.download = `snapshot_${session.ip.replace(/\./g, '_')}_${Date.now()}.jpg`;
    link.target = '_blank';
    link.click();
  };

  const isAdmin = session?.capabilities.user_permission === 'admin';

  return (
    <div className="fixed inset-0 z-50 bg-black/60 dark:bg-black/85 backdrop-blur-sm flex items-center justify-center p-4">
      <div className="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-2xl shadow-2xl w-full max-w-5xl max-h-[92vh] flex flex-col overflow-hidden animate-in fade-in zoom-in-95 duration-150 transition-colors">
        {/* Modal Top Header */}
        <div className="px-6 py-3.5 bg-slate-50 dark:bg-slate-950/90 border-b border-slate-200 dark:border-slate-800 flex items-center justify-between shrink-0">
          <div className="flex items-center gap-3">
            <div className="h-9 w-9 rounded-xl bg-sky-50 dark:bg-sky-500/20 border border-sky-200 dark:border-sky-500/30 flex items-center justify-center text-sky-600 dark:text-sky-400">
              <Eye className="h-5 w-5" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h3 className="text-base font-bold text-slate-800 dark:text-white tracking-tight">
                  Quick Viewer — {device.hardware_model || device.device_type_label}
                </h3>
                <span className="px-2 py-0.5 rounded-md text-[10px] font-mono font-bold bg-sky-50 dark:bg-sky-500/20 text-sky-600 dark:text-sky-300 border border-sky-200 dark:border-sky-500/30">
                  {device.ip}
                </span>
                {session && (
                  <span
                    className={`px-2 py-0.5 rounded-md text-[10px] font-bold border ${
                      isAdmin
                        ? 'bg-emerald-50 dark:bg-emerald-500/15 text-emerald-700 dark:text-emerald-300 border-emerald-200 dark:border-emerald-500/30'
                        : 'bg-amber-50 dark:bg-amber-500/15 text-amber-700 dark:text-amber-300 border-amber-200 dark:border-amber-500/30'
                    }`}
                  >
                    {isAdmin ? 'Privilégio: ADMIN' : 'Privilégio: VIEW ONLY'}
                  </span>
                )}
              </div>
              <p className="text-xs text-slate-500 dark:text-slate-400 font-sans">
                {device.brand} • {device.device_type_label} • Porta RTSP {device.rtsp_port || 554}
              </p>
            </div>
          </div>

          <button
            onClick={onClose}
            className="p-1.5 rounded-lg text-slate-400 hover:text-slate-700 dark:hover:text-white hover:bg-slate-100 dark:hover:bg-slate-800 transition"
          >
            <X className="h-5 w-5" />
          </button>
        </div>

        {/* Modal Body */}
        <div className="flex-1 min-h-0 overflow-y-auto">
          {step === 'auth' ? (
            /* STEP 1: AUTHENTICATION DIALOG */
            <div className="p-8 max-w-md mx-auto space-y-6">
              <div className="text-center space-y-2">
                <div className="h-12 w-12 rounded-2xl bg-sky-50 dark:bg-sky-500/10 border border-sky-200 dark:border-sky-500/20 text-sky-600 dark:text-sky-400 flex items-center justify-center mx-auto mb-3">
                  <Lock className="h-6 w-6" />
                </div>
                <h4 className="text-lg font-bold text-slate-800 dark:text-white">Autenticação do Dispositivo</h4>
                <p className="text-xs text-slate-500 dark:text-slate-400">
                  Informe o usuário e senha da câmera para autenticar (Digest/Basic Auth), detectar recursos e abrir o vídeo ao vivo:
                </p>
              </div>

              {connectError && (
                <div className="p-3.5 rounded-xl bg-rose-50 dark:bg-rose-500/15 border border-rose-200 dark:border-rose-500/30 text-rose-700 dark:text-rose-300 text-xs flex items-start gap-2.5">
                  <AlertTriangle className="h-4 w-4 shrink-0 mt-0.5" />
                  <span>{connectError}</span>
                </div>
              )}

              <form onSubmit={handleConnect} className="space-y-4">
                <div>
                  <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 uppercase tracking-wider mb-1.5">
                    Usuário
                  </label>
                  <input
                    type="text"
                    required
                    value={username}
                    onChange={(e) => setUsername(e.target.value)}
                    placeholder="admin"
                    className="w-full bg-slate-50 dark:bg-slate-950 border border-slate-300 dark:border-slate-800 rounded-xl px-4 py-2.5 text-sm text-slate-900 dark:text-white focus:outline-none focus:border-sky-500 font-mono"
                  />
                </div>

                <div>
                  <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 uppercase tracking-wider mb-1.5">
                    Senha do Dispositivo
                  </label>
                  <div className="relative">
                    <input
                      type={showPassword ? 'text' : 'password'}
                      value={password}
                      onChange={(e) => setPassword(e.target.value)}
                      placeholder="Digite a senha"
                      className="w-full bg-slate-50 dark:bg-slate-950 border border-slate-300 dark:border-slate-800 rounded-xl pl-4 pr-11 py-2.5 text-sm text-slate-900 dark:text-white focus:outline-none focus:border-sky-500 font-mono"
                    />
                    <button
                      type="button"
                      onClick={() => setShowPassword(!showPassword)}
                      className="absolute right-3 top-1/2 -translate-y-1/2 text-slate-400 hover:text-slate-600 dark:hover:text-slate-200"
                    >
                      {showPassword ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                    </button>
                  </div>
                </div>

                <div className="pt-2 flex items-center justify-end gap-3">
                  <button
                    type="button"
                    onClick={onClose}
                    className="px-4 py-2.5 rounded-xl bg-slate-100 dark:bg-slate-800 hover:bg-slate-200 dark:hover:bg-slate-700 text-slate-700 dark:text-slate-300 text-xs font-semibold transition"
                  >
                    Cancelar
                  </button>
                  <button
                    type="submit"
                    disabled={isConnecting}
                    className="px-6 py-2.5 rounded-xl bg-sky-600 hover:bg-sky-500 text-white text-xs font-bold shadow-lg shadow-sky-500/20 transition flex items-center gap-2 disabled:opacity-50"
                  >
                    {isConnecting ? (
                      <>
                        <Loader2 className="h-4 w-4 animate-spin" />
                        <span>Autenticando & Detectando...</span>
                      </>
                    ) : (
                      <>
                        <Radio className="h-4 w-4" />
                        <span>Conectar & Visualizar</span>
                      </>
                    )}
                  </button>
                </div>
              </form>
            </div>
          ) : (
            /* STEP 2: LIVE QUICK VIEWER & CONFIGURATION PANEL */
            session && (
              <div className="p-6 grid grid-cols-1 lg:grid-cols-12 gap-6">
                {/* LEFT COLUMN: LIVE VIDEO CANVAS & METRICS (7 Cols) */}
                <div className="lg:col-span-7 space-y-4">
                  {/* Video Canvas Container */}
                  <div
                    ref={videoContainerRef}
                    className="relative aspect-video bg-black rounded-xl overflow-hidden border border-slate-200 dark:border-slate-800 flex flex-col justify-between group shadow-xl"
                  >
                    {/* Top Overlay */}
                    <div className="absolute top-0 inset-x-0 z-20 bg-gradient-to-b from-black/80 via-black/40 to-transparent p-3 flex items-center justify-between pointer-events-none">
                      <div className="flex items-center gap-2">
                        <span className="h-2.5 w-2.5 rounded-full bg-emerald-500 shadow-sm shadow-emerald-500/50 animate-pulse" />
                        <span className="text-xs font-bold text-white drop-shadow">
                          {session.device_name || session.ip}
                        </span>
                      </div>

                      <div className="flex items-center gap-2 text-[11px] font-mono text-slate-300 drop-shadow">
                        {session.metrics.codec && (
                          <span className="bg-black/60 px-2 py-0.5 rounded border border-white/10 text-emerald-400 font-bold">
                            {session.metrics.codec}
                          </span>
                        )}
                        {session.metrics.resolution && (
                          <span className="bg-black/60 px-2 py-0.5 rounded border border-white/10">
                            {session.metrics.resolution}
                          </span>
                        )}
                        {session.metrics.fps && (
                          <span className="bg-black/60 px-2 py-0.5 rounded border border-white/10 text-sky-400 font-bold">
                            {session.metrics.fps} FPS
                          </span>
                        )}
                      </div>
                    </div>

                    {/* Stream Content */}
                    <div className="relative flex-1 w-full h-full bg-slate-950 flex items-center justify-center overflow-hidden">
                      {!imageError ? (
                        <img
                          src={`${session.local_mjpeg_url}?t=${reloadKey}`}
                          alt={session.device_name}
                          onError={() => setImageError(true)}
                          className="w-full h-full object-contain"
                        />
                      ) : (
                        <div className="p-6 text-center space-y-2">
                          <AlertTriangle className="h-8 w-8 text-rose-400 mx-auto" />
                          <p className="text-sm font-bold text-rose-300">Fluxo de Vídeo Indisponível</p>
                          <p className="text-xs text-slate-400 font-mono">
                            {session.metrics.message || 'Tentando reestabelecer conexão RTSP...'}
                          </p>
                          <button
                            onClick={handleReconnectLive}
                            className="px-3.5 py-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 text-white text-xs font-semibold inline-flex items-center gap-1.5 transition"
                          >
                            <RefreshCw className="h-3.5 w-3.5 text-sky-400" />
                            <span>Reconectar</span>
                          </button>
                        </div>
                      )}
                    </div>

                    {/* Bottom Controls Bar */}
                    <div className="absolute bottom-0 inset-x-0 z-20 bg-gradient-to-t from-black/80 via-black/40 to-transparent p-3 flex items-center justify-between opacity-0 group-hover:opacity-100 transition-opacity">
                      <span className="text-[11px] font-mono text-slate-400">
                        {session.capabilities.protocol_used} ({session.capabilities.auth_type})
                      </span>

                      <div className="flex items-center gap-2 pointer-events-auto">
                        <button
                          onClick={handleSnapshot}
                          className="px-2.5 py-1 rounded-lg bg-black/70 hover:bg-black text-white text-xs font-semibold border border-white/10 inline-flex items-center gap-1.5 transition"
                          title="Capturar Foto"
                        >
                          <CameraIcon className="h-3.5 w-3.5 text-sky-400" />
                          <span>Snapshot</span>
                        </button>
                        <button
                          onClick={handleReconnectLive}
                          className="p-1.5 rounded-lg bg-black/70 hover:bg-black text-white border border-white/10 transition"
                          title="Reconectar Stream"
                        >
                          <RefreshCw className="h-3.5 w-3.5" />
                        </button>
                        <button
                          onClick={toggleFullscreen}
                          className="p-1.5 rounded-lg bg-black/70 hover:bg-black text-white border border-white/10 transition"
                          title={isFullscreen ? 'Sair da Tela Cheia' : 'Tela Cheia'}
                        >
                          {isFullscreen ? <Minimize2 className="h-3.5 w-3.5" /> : <Maximize2 className="h-3.5 w-3.5" />}
                        </button>
                      </div>
                    </div>
                  </div>

                  {/* Diagnostic Summary Bar */}
                  <div className="bg-slate-50 dark:bg-slate-950/80 border border-slate-200 dark:border-slate-800/80 rounded-xl p-3.5 space-y-2 text-xs font-mono">
                    <div className="flex items-center justify-between text-slate-600 dark:text-slate-400">
                      <span className="font-bold text-slate-800 dark:text-slate-200 flex items-center gap-1.5">
                        <Activity className="h-3.5 w-3.5 text-emerald-600 dark:text-emerald-400" />
                        Diagnóstico em Tempo Real:
                      </span>
                      <span className="text-emerald-600 dark:text-emerald-400 font-semibold">● Conexão Estável</span>
                    </div>
                    <div className="grid grid-cols-2 sm:grid-cols-4 gap-2 text-[11px] pt-1">
                      <div className="bg-white dark:bg-slate-900/60 p-2 rounded-lg border border-slate-200 dark:border-slate-800">
                        <span className="text-slate-400 dark:text-slate-500 block">Rede IP</span>
                        <span className="font-bold text-slate-800 dark:text-slate-200">{session.ip}:{session.rtsp_port}</span>
                      </div>
                      <div className="bg-white dark:bg-slate-900/60 p-2 rounded-lg border border-slate-200 dark:border-slate-800">
                        <span className="text-slate-400 dark:text-slate-500 block">Autenticação</span>
                        <span className="font-bold text-emerald-600 dark:text-emerald-300">{session.capabilities.auth_type} Auth</span>
                      </div>
                      <div className="bg-white dark:bg-slate-900/60 p-2 rounded-lg border border-slate-200 dark:border-slate-800">
                        <span className="text-slate-400 dark:text-slate-500 block">Resolução / FPS</span>
                        <span className="font-bold text-sky-600 dark:text-sky-300">{session.metrics.resolution || '1080p'} @ {session.metrics.fps || 25}fps</span>
                      </div>
                      <div className="bg-white dark:bg-slate-900/60 p-2 rounded-lg border border-slate-200 dark:border-slate-800">
                        <span className="text-slate-400 dark:text-slate-500 block">Latência</span>
                        <span className="font-bold text-slate-800 dark:text-slate-200">{session.metrics.latency_ms || 12} ms</span>
                      </div>
                    </div>
                  </div>
                </div>

                {/* RIGHT COLUMN: DEVICE INFO & FAST CONFIGURATION (5 Cols) */}
                <div className="lg:col-span-5 space-y-4">
                  {/* Device Info Card */}
                  <div className="bg-slate-50 dark:bg-slate-950/80 border border-slate-200 dark:border-slate-800/80 rounded-xl p-4 space-y-3">
                    <h5 className="text-xs font-bold text-slate-800 dark:text-white uppercase tracking-wider flex items-center gap-1.5">
                      <Tv className="h-4 w-4 text-sky-600 dark:text-sky-400" />
                      Informações do Dispositivo
                    </h5>

                    <div className="space-y-1.5 text-xs">
                      <div className="flex justify-between py-1 border-b border-slate-200 dark:border-slate-800/60">
                        <span className="text-slate-500 dark:text-slate-400">Fabricante:</span>
                        <span className="font-bold text-slate-800 dark:text-white">{session.brand}</span>
                      </div>
                      <div className="flex justify-between py-1 border-b border-slate-200 dark:border-slate-800/60">
                        <span className="text-slate-500 dark:text-slate-400">Modelo:</span>
                        <span className="font-bold text-sky-600 dark:text-sky-300 font-mono">{session.hardware_model}</span>
                      </div>
                      {session.serial_number && (
                        <div className="flex justify-between py-1 border-b border-slate-200 dark:border-slate-800/60">
                          <span className="text-slate-500 dark:text-slate-400">Número de Série:</span>
                          <span className="font-mono text-slate-700 dark:text-slate-200 text-[11px]">{session.serial_number}</span>
                        </div>
                      )}
                      {session.firmware_version && (
                        <div className="flex justify-between py-1 border-b border-slate-200 dark:border-slate-800/60">
                          <span className="text-slate-500 dark:text-slate-400">Firmware:</span>
                          <span className="font-mono text-slate-700 dark:text-slate-200 text-[11px]">{session.firmware_version}</span>
                        </div>
                      )}
                      {session.mac_address && (
                        <div className="flex justify-between py-1">
                          <span className="text-slate-500 dark:text-slate-400">MAC Address:</span>
                          <span className="font-mono text-slate-600 dark:text-slate-300 text-[11px]">{session.mac_address}</span>
                        </div>
                      )}
                    </div>
                  </div>

                  {/* Configuration Box */}
                  <div className="bg-slate-50 dark:bg-slate-950/80 border border-slate-200 dark:border-slate-800/80 rounded-xl p-4 space-y-4">
                    <div className="flex items-center justify-between">
                      <h5 className="text-xs font-bold text-slate-800 dark:text-white uppercase tracking-wider flex items-center gap-1.5">
                        <Sliders className="h-4 w-4 text-sky-600 dark:text-sky-400" />
                        Configuração Rápida em Campo
                      </h5>
                    </div>

                    {!isAdmin && (
                      <div className="p-3 rounded-lg bg-amber-50 dark:bg-amber-500/15 border border-amber-200 dark:border-amber-500/30 text-amber-800 dark:text-amber-300 text-xs">
                        ⚠️ Usuário autenticado sem permissão administrativa para alterar configurações no dispositivo.
                      </div>
                    )}

                    {/* 1. Device Name Form */}
                    <form onSubmit={handleSaveDeviceName} className="space-y-2">
                      <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300">
                        Device Name (Nome Lógico)
                      </label>
                      <div className="flex gap-2">
                        <input
                          type="text"
                          disabled={!isAdmin || isSavingName}
                          value={editDeviceName}
                          onChange={(e) => setEditDeviceName(e.target.value)}
                          placeholder="Ex: CAM-ENTRADA-01"
                          className="flex-1 bg-white dark:bg-slate-900 border border-slate-300 dark:border-slate-800 rounded-lg px-3 py-1.5 text-xs text-slate-900 dark:text-white focus:outline-none focus:border-sky-500 disabled:opacity-50 font-sans"
                        />
                        <button
                          type="submit"
                          disabled={!isAdmin || isSavingName || !editDeviceName.trim()}
                          className="px-3 py-1.5 rounded-lg bg-sky-600 hover:bg-sky-500 text-white text-xs font-bold shadow transition flex items-center gap-1.5 disabled:opacity-40"
                        >
                          {isSavingName ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <Save className="h-3.5 w-3.5" />}
                          <span>Salvar</span>
                        </button>
                      </div>
                      {nameSaveMsg && (
                        <p className={`text-[11px] ${nameSaveMsg.success ? 'text-emerald-600 dark:text-emerald-400' : 'text-rose-600 dark:text-rose-400'}`}>
                          {nameSaveMsg.text}
                        </p>
                      )}
                    </form>

                    {/* 2. OSD Form */}
                    <form onSubmit={handleSaveOsd} className="space-y-2">
                      <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300">
                        OSD (Texto Apresentado Sobre o Vídeo)
                      </label>
                      <div className="flex gap-2">
                        <input
                          type="text"
                          disabled={!isAdmin || isSavingOsd || !session.capabilities.can_change_osd}
                          value={editOsd}
                          onChange={(e) => setEditOsd(e.target.value)}
                          placeholder="Ex: ENTRADA PRINCIPAL"
                          className="flex-1 bg-white dark:bg-slate-900 border border-slate-300 dark:border-slate-800 rounded-lg px-3 py-1.5 text-xs text-slate-900 dark:text-white focus:outline-none focus:border-sky-500 disabled:opacity-50 font-sans"
                        />
                        <button
                          type="submit"
                          disabled={!isAdmin || isSavingOsd || !session.capabilities.can_change_osd}
                          className="px-3 py-1.5 rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-bold shadow transition flex items-center gap-1.5 disabled:opacity-40"
                        >
                          {isSavingOsd ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <Check className="h-3.5 w-3.5" />}
                          <span>Aplicar OSD</span>
                        </button>
                      </div>
                      {!session.capabilities.can_change_osd && (
                        <p className="text-[10px] text-slate-400 dark:text-slate-500 italic">
                          Este dispositivo não disponibiliza alteração de OSD pela interface suportada.
                        </p>
                      )}
                      {osdSaveMsg && (
                        <p className={`text-[11px] ${osdSaveMsg.success ? 'text-emerald-600 dark:text-emerald-400' : 'text-rose-600 dark:text-rose-400'}`}>
                          {osdSaveMsg.text}
                        </p>
                      )}
                    </form>

                    {/* Quick Add Button */}
                    {onAddAsCamera && !device.is_already_added && (
                      <div className="pt-2 border-t border-slate-200 dark:border-slate-800">
                        <button
                          onClick={() => {
                            onClose();
                            onAddAsCamera({
                              name: session.device_name || device.name,
                              host: session.ip,
                              username,
                              password,
                              rtsp_port: session.rtsp_port,
                              stream_profile: 'main',
                            });
                          }}
                          className="w-full py-2 rounded-xl bg-sky-50 dark:bg-sky-500/20 hover:bg-sky-100 dark:hover:bg-sky-500/30 text-sky-600 dark:text-sky-300 border border-sky-200 dark:border-sky-500/40 text-xs font-bold flex items-center justify-center gap-2 transition shadow-sm"
                        >
                          <PlusCircle className="h-4 w-4" />
                          <span>Cadastrar Permanentemente no OnliView</span>
                        </button>
                      </div>
                    )}
                  </div>
                </div>
              </div>
            )
          )}
        </div>
      </div>
    </div>
  );
};
