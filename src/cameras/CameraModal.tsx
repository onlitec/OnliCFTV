import React, { useState, useEffect } from 'react';
import { X, Check, AlertTriangle, Eye, EyeOff, Loader2, PlayCircle, ShieldCheck } from 'lucide-react';
import type { Camera, CreateCameraInput, CameraConnectionTestResult } from '@/types';
import { api } from '@/services/api';

interface CameraModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSaved: () => void;
  cameraToEdit?: Camera | null;
}

export const CameraModal: React.FC<CameraModalProps> = ({
  isOpen,
  onClose,
  onSaved,
  cameraToEdit,
}) => {
  const [name, setName] = useState('');
  const [host, setHost] = useState('');
  const [username, setUsername] = useState('admin');
  const [password, setPassword] = useState('');
  const [showPassword, setShowPassword] = useState(false);
  const [rtspPort, setRtspPort] = useState(554);
  const [streamProfile, setStreamProfile] = useState<'main' | 'sub' | 'custom'>('main');
  const [customRtspUrl, setCustomRtspUrl] = useState('');
  const [enabled, setEnabled] = useState(true);

  const [isTesting, setIsTesting] = useState(false);
  const [testResult, setTestResult] = useState<CameraConnectionTestResult | null>(null);
  const [isSaving, setIsSaving] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  useEffect(() => {
    if (cameraToEdit) {
      setName(cameraToEdit.name);
      setHost(cameraToEdit.host);
      setUsername(cameraToEdit.username);
      setPassword(''); // keep blank unless user types new
      setRtspPort(cameraToEdit.rtsp_port);
      setStreamProfile((cameraToEdit.stream_profile as any) || 'main');
      setCustomRtspUrl(cameraToEdit.rtsp_url);
      setEnabled(cameraToEdit.enabled);
    } else {
      setName('');
      setHost('');
      setUsername('admin');
      setPassword('');
      setRtspPort(554);
      setStreamProfile('main');
      setCustomRtspUrl('');
      setEnabled(true);
    }
    setTestResult(null);
    setErrorMsg(null);
  }, [cameraToEdit, isOpen]);

  if (!isOpen) return null;

  const handleTestConnection = async () => {
    if (!host) {
      setErrorMsg('Informe o IP ou Hostname da câmera.');
      return;
    }
    setIsTesting(true);
    setErrorMsg(null);
    setTestResult(null);

    try {
      const input: CreateCameraInput = {
        name: name || 'Câmera Teste',
        host,
        username,
        password: password || undefined,
        rtsp_port: Number(rtspPort) || 554,
        stream_profile: streamProfile,
        rtsp_url: streamProfile === 'custom' ? customRtspUrl : undefined,
      };

      const res = await api.testCameraConnection(input);
      setTestResult(res);
    } catch (err: any) {
      setTestResult({
        success: false,
        message: err?.toString() || 'Erro inesperado ao testar conexão',
      });
    } finally {
      setIsTesting(false);
    }
  };

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim() || !host.trim()) {
      setErrorMsg('Preencha o nome e o IP/Hostname da câmera.');
      return;
    }

    setIsSaving(true);
    setErrorMsg(null);

    try {
      if (cameraToEdit) {
        await api.updateCamera({
          id: cameraToEdit.id,
          name,
          host,
          username,
          password: password || undefined,
          rtsp_port: Number(rtspPort),
          stream_profile: streamProfile,
          rtsp_url: streamProfile === 'custom' ? customRtspUrl : undefined,
          enabled,
        });
      } else {
        await api.createCamera({
          name,
          host,
          username,
          password: password || undefined,
          rtsp_port: Number(rtspPort),
          stream_profile: streamProfile,
          rtsp_url: streamProfile === 'custom' ? customRtspUrl : undefined,
          enabled,
        });
      }
      onSaved();
      onClose();
    } catch (err: any) {
      setErrorMsg(err?.toString() || 'Falha ao salvar câmera no banco de dados');
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm flex items-center justify-center p-4">
      <div className="bg-slate-900 border border-slate-800 rounded-xl shadow-2xl w-full max-w-lg overflow-hidden animate-in fade-in zoom-in-95 duration-150">
        {/* Header */}
        <div className="px-6 py-4 border-b border-slate-800 flex items-center justify-between bg-slate-950/40">
          <h3 className="text-base font-bold text-white flex items-center gap-2">
            {cameraToEdit ? 'Editar Câmera' : 'Adicionar Nova Câmera'}
          </h3>
          <button
            onClick={onClose}
            className="p-1 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800"
          >
            <X className="h-5 w-5" />
          </button>
        </div>

        {/* Form */}
        <form onSubmit={handleSave} className="p-6 space-y-4 max-h-[80vh] overflow-y-auto">
          {errorMsg && (
            <div className="p-3 rounded-lg bg-rose-500/15 border border-rose-500/30 text-rose-300 text-xs flex items-center gap-2">
              <AlertTriangle className="h-4 w-4 shrink-0" />
              <span>{errorMsg}</span>
            </div>
          )}

          <div>
            <label className="block text-xs font-semibold text-slate-300 uppercase tracking-wider mb-1.5">
              Nome de Identificação *
            </label>
            <input
              type="text"
              required
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Ex: Câmera Portaria Principal"
              className="w-full bg-slate-950 border border-slate-800 rounded-lg px-3.5 py-2 text-sm text-white focus:outline-none focus:border-sky-500 placeholder-slate-500"
            />
          </div>

          <div className="grid grid-cols-3 gap-3">
            <div className="col-span-2">
              <label className="block text-xs font-semibold text-slate-300 uppercase tracking-wider mb-1.5">
                IP ou Hostname *
              </label>
              <input
                type="text"
                required
                value={host}
                onChange={(e) => setHost(e.target.value)}
                placeholder="Ex: 172.20.120.67"
                className="w-full bg-slate-950 border border-slate-800 rounded-lg px-3.5 py-2 text-sm text-white focus:outline-none focus:border-sky-500 placeholder-slate-500 font-mono"
              />
            </div>
            <div>
              <label className="block text-xs font-semibold text-slate-300 uppercase tracking-wider mb-1.5">
                Porta RTSP
              </label>
              <input
                type="number"
                value={rtspPort}
                onChange={(e) => setRtspPort(Number(e.target.value))}
                placeholder="554"
                className="w-full bg-slate-950 border border-slate-800 rounded-lg px-3.5 py-2 text-sm text-white focus:outline-none focus:border-sky-500 font-mono"
              />
            </div>
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className="block text-xs font-semibold text-slate-300 uppercase tracking-wider mb-1.5">
                Usuário RTSP
              </label>
              <input
                type="text"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                placeholder="admin"
                className="w-full bg-slate-950 border border-slate-800 rounded-lg px-3.5 py-2 text-sm text-white focus:outline-none focus:border-sky-500"
              />
            </div>
            <div>
              <label className="block text-xs font-semibold text-slate-300 uppercase tracking-wider mb-1.5">
                Senha RTSP
              </label>
              <div className="relative">
                <input
                  type={showPassword ? 'text' : 'password'}
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  placeholder={cameraToEdit ? '(Inalterada)' : 'Senha da câmera'}
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
          </div>

          <div>
            <label className="block text-xs font-semibold text-slate-300 uppercase tracking-wider mb-1.5">
              Perfil do Stream (Hikvision / Padrão)
            </label>
            <div className="grid grid-cols-3 gap-2">
              <button
                type="button"
                onClick={() => setStreamProfile('main')}
                className={`py-2 px-3 rounded-lg text-xs font-medium border text-center transition ${
                  streamProfile === 'main'
                    ? 'bg-sky-500/20 border-sky-500/50 text-sky-300'
                    : 'bg-slate-950 border-slate-800 text-slate-400 hover:border-slate-700'
                }`}
              >
                Principal (101)
              </button>
              <button
                type="button"
                onClick={() => setStreamProfile('sub')}
                className={`py-2 px-3 rounded-lg text-xs font-medium border text-center transition ${
                  streamProfile === 'sub'
                    ? 'bg-sky-500/20 border-sky-500/50 text-sky-300'
                    : 'bg-slate-950 border-slate-800 text-slate-400 hover:border-slate-700'
                }`}
              >
                Secundário (102)
              </button>
              <button
                type="button"
                onClick={() => setStreamProfile('custom')}
                className={`py-2 px-3 rounded-lg text-xs font-medium border text-center transition ${
                  streamProfile === 'custom'
                    ? 'bg-sky-500/20 border-sky-500/50 text-sky-300'
                    : 'bg-slate-950 border-slate-800 text-slate-400 hover:border-slate-700'
                }`}
              >
                Personalizado
              </button>
            </div>
          </div>

          {streamProfile === 'custom' && (
            <div>
              <label className="block text-xs font-semibold text-slate-300 uppercase tracking-wider mb-1.5">
                URI / URL RTSP Personalizada
              </label>
              <input
                type="text"
                value={customRtspUrl}
                onChange={(e) => setCustomRtspUrl(e.target.value)}
                placeholder="rtsp://172.20.120.67:554/Streaming/Channels/101"
                className="w-full bg-slate-950 border border-slate-800 rounded-lg px-3.5 py-2 text-sm text-white focus:outline-none focus:border-sky-500 font-mono placeholder-slate-600"
              />
            </div>
          )}

          {/* Test Connection Button & Output */}
          <div className="pt-2 border-t border-slate-800/80">
            <button
              type="button"
              onClick={handleTestConnection}
              disabled={isTesting || !host}
              className="w-full py-2.5 px-4 rounded-lg bg-slate-800 hover:bg-slate-700 border border-slate-700 text-slate-200 hover:text-white text-xs font-semibold flex items-center justify-center gap-2 transition disabled:opacity-50"
            >
              {isTesting ? (
                <>
                  <Loader2 className="h-4 w-4 animate-spin text-sky-400" />
                  <span>Testando Conexão RTSP...</span>
                </>
              ) : (
                <>
                  <PlayCircle className="h-4 w-4 text-sky-400" />
                  <span>Testar Conexão e Detectar Codec</span>
                </>
              )}
            </button>

            {testResult && (
              <div
                className={`mt-3 p-3.5 rounded-lg border text-xs ${
                  testResult.success
                    ? 'bg-emerald-500/10 border-emerald-500/30 text-emerald-300'
                    : 'bg-rose-500/10 border-rose-500/30 text-rose-300'
                }`}
              >
                <div className="flex items-center gap-2 font-semibold mb-1">
                  {testResult.success ? (
                    <ShieldCheck className="h-4 w-4 text-emerald-400" />
                  ) : (
                    <AlertTriangle className="h-4 w-4 text-rose-400" />
                  )}
                  <span>{testResult.message}</span>
                </div>
                {testResult.success && (
                  <div className="grid grid-cols-3 gap-2 mt-2 pt-2 border-t border-emerald-500/20 text-[11px] font-mono">
                    <div>Codec: <strong className="text-white">{testResult.codec || 'N/A'}</strong></div>
                    <div>Resolução: <strong className="text-white">{testResult.resolution || 'N/A'}</strong></div>
                    <div>Latência: <strong className="text-white">{testResult.latency_ms ? `${testResult.latency_ms}ms` : 'N/A'}</strong></div>
                  </div>
                )}
              </div>
            )}
          </div>

          {/* Footer Actions */}
          <div className="pt-3 border-t border-slate-800 flex items-center justify-end gap-3">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-300 text-sm font-medium transition"
            >
              Cancelar
            </button>
            <button
              type="submit"
              disabled={isSaving}
              className="px-5 py-2 rounded-lg bg-sky-600 hover:bg-sky-500 text-white text-sm font-semibold shadow-md shadow-sky-950 transition flex items-center gap-2 disabled:opacity-50"
            >
              {isSaving ? <Loader2 className="h-4 w-4 animate-spin" /> : <Check className="h-4 w-4" />}
              <span>{cameraToEdit ? 'Atualizar Câmera' : 'Salvar Câmera'}</span>
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
