import React, { useState, useEffect } from 'react';
import {
  X,
  Search,
  RefreshCw,
  Plus,
  Check,
  CheckCircle2,
  AlertTriangle,
  Eye,
  EyeOff,
  Loader2,
  Camera,
  Layers,
  Radio,
} from 'lucide-react';
import type { DiscoveredDevice, BatchCreateCamerasInput, CreateCameraInput } from '@/types';
import { api } from '@/services/api';

interface DiscoveryModalProps {
  isOpen: boolean;
  onClose: () => void;
  onAdded: () => void;
  onAddSingle: (prefill: CreateCameraInput) => void;
}

export const DiscoveryModal: React.FC<DiscoveryModalProps> = ({
  isOpen,
  onClose,
  onAdded,
  onAddSingle,
}) => {
  const [devices, setDevices] = useState<DiscoveredDevice[]>([]);
  const [isScanning, setIsScanning] = useState(false);
  const [selectedIps, setSelectedIps] = useState<Set<string>>(new Set());

  // Batch Dialog State
  const [isBatchOpen, setIsBatchOpen] = useState(false);
  const [batchUsername, setBatchUsername] = useState('admin');
  const [batchPassword, setBatchPassword] = useState('');
  const [showPassword, setShowPassword] = useState(false);
  const [batchProfile, setBatchProfile] = useState<'main' | 'sub'>('main');
  const [isSavingBatch, setIsSavingBatch] = useState(false);
  const [batchError, setBatchError] = useState<string | null>(null);

  const scanDevices = async () => {
    setIsScanning(true);
    try {
      const found = await api.discoverDevices();
      setDevices(found);
      // Auto-select only devices not yet added
      const newIps = new Set(found.filter((d) => !d.is_already_added).map((d) => d.ip));
      setSelectedIps(newIps);
    } catch (e) {
      console.error('Failed to discover devices:', e);
    } finally {
      setIsScanning(false);
    }
  };

  useEffect(() => {
    if (isOpen) {
      scanDevices();
    }
  }, [isOpen]);

  if (!isOpen) return null;

  const toggleSelect = (ip: string) => {
    const next = new Set(selectedIps);
    if (next.has(ip)) {
      next.delete(ip);
    } else {
      next.add(ip);
    }
    setSelectedIps(next);
  };

  const toggleSelectAll = () => {
    const unadded = devices.filter((d) => !d.is_already_added);
    if (selectedIps.size === unadded.length && unadded.length > 0) {
      setSelectedIps(new Set());
    } else {
      setSelectedIps(new Set(unadded.map((d) => d.ip)));
    }
  };

  const handleConfirmBatch = async (e: React.FormEvent) => {
    e.preventDefault();
    if (selectedIps.size === 0) return;

    setIsSavingBatch(true);
    setBatchError(null);

    try {
      const selectedDevices = devices.filter((d) => selectedIps.has(d.ip));
      const input: BatchCreateCamerasInput = {
        devices: selectedDevices.map((d) => ({
          name: d.name,
          host: d.ip,
          rtsp_port: d.rtsp_port || 554,
        })),
        username: batchUsername.trim() || 'admin',
        password: batchPassword || undefined,
        stream_profile: batchProfile,
      };

      await api.createCamerasBatch(input);
      setIsBatchOpen(false);
      onAdded();
      onClose();
    } catch (err: any) {
      setBatchError(err?.toString() || 'Erro ao adicionar câmeras em lote');
    } finally {
      setIsSavingBatch(false);
    }
  };

  const unaddedCount = devices.filter((d) => !d.is_already_added).length;

  return (
    <div className="fixed inset-0 z-50 bg-black/75 backdrop-blur-sm flex items-center justify-center p-4">
      <div className="bg-slate-900 border border-slate-800 rounded-xl shadow-2xl w-full max-w-3xl overflow-hidden animate-in fade-in zoom-in-95 duration-150 flex flex-col max-h-[85vh]">
        {/* Modal Header */}
        <div className="px-6 py-4 border-b border-slate-800 flex items-center justify-between bg-slate-950/60 shrink-0">
          <div className="flex items-center gap-3">
            <div className="h-9 w-9 rounded-lg bg-sky-500/20 border border-sky-500/30 flex items-center justify-center text-sky-400">
              <Search className="h-5 w-5" />
            </div>
            <div>
              <h3 className="text-base font-bold text-white flex items-center gap-2">
                Busca de Dispositivos na Rede Local
              </h3>
              <p className="text-xs text-slate-400">
                Detecção automática de câmeras IP, NVRs e videoporteiros ONVIF / Hikvision
              </p>
            </div>
          </div>

          <div className="flex items-center gap-2">
            <button
              onClick={scanDevices}
              disabled={isScanning}
              className="px-3 py-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-200 border border-slate-700 text-xs font-semibold flex items-center gap-1.5 transition disabled:opacity-50"
            >
              <RefreshCw className={`h-3.5 w-3.5 ${isScanning ? 'animate-spin text-sky-400' : ''}`} />
              <span>{isScanning ? 'Buscando...' : 'Buscar Novamente'}</span>
            </button>
            <button
              onClick={onClose}
              className="p-1.5 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800"
            >
              <X className="h-5 w-5" />
            </button>
          </div>
        </div>

        {/* Content Area */}
        <div className="flex-1 overflow-y-auto p-6 space-y-4">
          {isScanning && devices.length === 0 ? (
            <div className="py-16 text-center space-y-3">
              <Loader2 className="h-10 w-10 text-sky-400 animate-spin mx-auto" />
              <h4 className="text-sm font-bold text-white">Varrendo a rede local (UDP Multicast)...</h4>
              <p className="text-xs text-slate-400 max-w-sm mx-auto">
                Escutando anúncios ONVIF WS-Discovery e respostas de equipamentos CFTV.
              </p>
            </div>
          ) : devices.length === 0 ? (
            <div className="py-12 text-center space-y-3 bg-slate-950/40 rounded-xl border border-dashed border-slate-800">
              <Camera className="h-10 w-10 text-slate-600 mx-auto" />
              <h4 className="text-sm font-bold text-white">Nenhum dispositivo encontrado</h4>
              <p className="text-xs text-slate-400 max-w-sm mx-auto">
                Certifique-se de que as câmeras estão ligadas e na mesma sub-rede IP (ex.: 172.20.120.x).
              </p>
              <button
                onClick={scanDevices}
                className="px-4 py-2 rounded-lg bg-sky-600 hover:bg-sky-500 text-white text-xs font-semibold inline-flex items-center gap-1.5 transition"
              >
                <RefreshCw className="h-3.5 w-3.5" />
                <span>Tentar Novamente</span>
              </button>
            </div>
          ) : (
            <div className="space-y-3">
              {/* Summary Bar */}
              <div className="flex items-center justify-between text-xs text-slate-400 bg-slate-950/40 px-3 py-2 rounded-lg border border-slate-800/80">
                <div className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    id="select-all"
                    checked={selectedIps.size === unaddedCount && unaddedCount > 0}
                    onChange={toggleSelectAll}
                    disabled={unaddedCount === 0}
                    className="rounded border-slate-700 bg-slate-900 text-sky-500 focus:ring-sky-500 h-4 w-4 cursor-pointer"
                  />
                  <label htmlFor="select-all" className="cursor-pointer font-medium text-slate-300">
                    Selecionar todos os novos ({unaddedCount})
                  </label>
                </div>
                <div>
                  Encontrados: <strong className="text-white">{devices.length}</strong> dispositivo(s)
                </div>
              </div>

              {/* Devices Table */}
              <div className="border border-slate-800 rounded-xl overflow-hidden bg-slate-950/60">
                <table className="w-full text-left text-xs font-mono">
                  <thead className="bg-slate-900/90 text-slate-400 uppercase text-[10px] tracking-wider border-b border-slate-800">
                    <tr>
                      <th className="px-4 py-3 w-10"></th>
                      <th className="px-4 py-3">Dispositivo / Nome</th>
                      <th className="px-4 py-3">Modelo</th>
                      <th className="px-4 py-3">IP : Porta</th>
                      <th className="px-4 py-3">Status</th>
                      <th className="px-4 py-3 text-right">Ações</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-slate-800/60">
                    {devices.map((dev) => {
                      const isSelected = selectedIps.has(dev.ip);
                      const isAdded = dev.is_already_added;

                      return (
                        <tr
                          key={dev.ip}
                          className={`hover:bg-slate-800/40 transition ${
                            isSelected ? 'bg-sky-500/5' : ''
                          }`}
                        >
                          <td className="px-4 py-3">
                            <input
                              type="checkbox"
                              checked={isSelected}
                              disabled={isAdded}
                              onChange={() => toggleSelect(dev.ip)}
                              className="rounded border-slate-700 bg-slate-900 text-sky-500 focus:ring-sky-500 h-4 w-4 cursor-pointer disabled:opacity-30"
                            />
                          </td>
                          <td className="px-4 py-3 font-sans">
                            <div className="font-bold text-white leading-tight">{dev.name}</div>
                            <div className="text-[10px] text-sky-400/80 font-mono mt-0.5">{dev.brand}</div>
                          </td>
                          <td className="px-4 py-3 text-slate-300 font-semibold">{dev.hardware_model}</td>
                          <td className="px-4 py-3 text-slate-200">
                            {dev.ip}:{dev.rtsp_port}
                          </td>
                          <td className="px-4 py-3">
                            {isAdded ? (
                              <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-semibold bg-emerald-500/20 text-emerald-300 border border-emerald-500/30">
                                <CheckCircle2 className="h-3 w-3 text-emerald-400" />
                                Cadastrada
                              </span>
                            ) : (
                              <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-semibold bg-sky-500/20 text-sky-300 border border-sky-500/30">
                                <Radio className="h-3 w-3 text-sky-400 animate-pulse" />
                                Disponível
                              </span>
                            )}
                          </td>
                          <td className="px-4 py-3 text-right">
                            {!isAdded ? (
                              <button
                                onClick={() => {
                                  onAddSingle({
                                    name: dev.name,
                                    host: dev.ip,
                                    username: 'admin',
                                    rtsp_port: dev.rtsp_port,
                                    stream_profile: 'main',
                                  });
                                  onClose();
                                }}
                                className="px-2.5 py-1 rounded bg-sky-600 hover:bg-sky-500 text-white text-xs font-semibold inline-flex items-center gap-1 transition shadow font-sans"
                                title="Adicionar com 1 clique"
                              >
                                <Plus className="h-3.5 w-3.5" />
                                <span>Adicionar</span>
                              </button>
                            ) : (
                              <span className="text-[11px] text-slate-500 italic">No banco</span>
                            )}
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            </div>
          )}
        </div>

        {/* Modal Footer / Batch Action Toolbar */}
        <div className="px-6 py-4 border-t border-slate-800 bg-slate-950 flex items-center justify-between shrink-0">
          <div className="text-xs text-slate-400">
            {selectedIps.size > 0 ? (
              <span className="text-sky-400 font-semibold">
                {selectedIps.size} câmera(s) selecionada(s)
              </span>
            ) : (
              <span>Selecione as câmeras para cadastrar em lote</span>
            )}
          </div>

          <div className="flex items-center gap-3">
            <button
              onClick={onClose}
              className="px-4 py-2 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-300 text-xs font-medium transition"
            >
              Fechar
            </button>
            <button
              onClick={() => setIsBatchOpen(true)}
              disabled={selectedIps.size === 0}
              className="px-5 py-2 rounded-lg bg-sky-600 hover:bg-sky-500 text-white text-xs font-bold shadow-lg shadow-sky-950 flex items-center gap-2 transition disabled:opacity-50 disabled:pointer-events-none"
            >
              <Layers className="h-4 w-4" />
              <span>Adicionar Selecionadas em Lote ({selectedIps.size})</span>
            </button>
          </div>
        </div>
      </div>

      {/* Nested Batch Password Modal */}
      {isBatchOpen && (
        <div className="fixed inset-0 z-60 bg-black/80 backdrop-blur-md flex items-center justify-center p-4">
          <div className="bg-slate-900 border border-slate-800 rounded-xl shadow-2xl w-full max-w-md overflow-hidden animate-in fade-in zoom-in-95 duration-150">
            <div className="px-6 py-4 border-b border-slate-800 flex items-center justify-between bg-slate-950/60">
              <h4 className="text-base font-bold text-white flex items-center gap-2">
                <Layers className="h-4 w-4 text-sky-400" />
                Cadastrar {selectedIps.size} Câmera(s) em Lote
              </h4>
              <button
                onClick={() => setIsBatchOpen(false)}
                className="p-1 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800"
              >
                <X className="h-5 w-5" />
              </button>
            </div>

            <form onSubmit={handleConfirmBatch} className="p-6 space-y-4">
              <p className="text-xs text-slate-300">
                Informe as credenciais de acesso padrão. Elas serão criptografadas e aplicadas a todas as câmeras selecionadas:
              </p>

              {batchError && (
                <div className="p-3 rounded-lg bg-rose-500/15 border border-rose-500/30 text-rose-300 text-xs flex items-center gap-2">
                  <AlertTriangle className="h-4 w-4 shrink-0" />
                  <span>{batchError}</span>
                </div>
              )}

              <div>
                <label className="block text-xs font-semibold text-slate-300 uppercase tracking-wider mb-1.5">
                  Usuário RTSP / ONVIF
                </label>
                <input
                  type="text"
                  required
                  value={batchUsername}
                  onChange={(e) => setBatchUsername(e.target.value)}
                  placeholder="admin"
                  className="w-full bg-slate-950 border border-slate-800 rounded-lg px-3.5 py-2 text-sm text-white focus:outline-none focus:border-sky-500"
                />
              </div>

              <div>
                <label className="block text-xs font-semibold text-slate-300 uppercase tracking-wider mb-1.5">
                  Senha das Câmeras (Digitada uma única vez)
                </label>
                <div className="relative">
                  <input
                    type={showPassword ? 'text' : 'password'}
                    value={batchPassword}
                    onChange={(e) => setBatchPassword(e.target.value)}
                    placeholder="Digite a senha para todas"
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

              <div>
                <label className="block text-xs font-semibold text-slate-300 uppercase tracking-wider mb-1.5">
                  Perfil de Stream Padrão
                </label>
                <div className="grid grid-cols-2 gap-2">
                  <button
                    type="button"
                    onClick={() => setBatchProfile('main')}
                    className={`py-2 px-3 rounded-lg text-xs font-medium border text-center transition ${
                      batchProfile === 'main'
                        ? 'bg-sky-500/20 border-sky-500/50 text-sky-300'
                        : 'bg-slate-950 border-slate-800 text-slate-400 hover:border-slate-700'
                    }`}
                  >
                    Principal (101)
                  </button>
                  <button
                    type="button"
                    onClick={() => setBatchProfile('sub')}
                    className={`py-2 px-3 rounded-lg text-xs font-medium border text-center transition ${
                      batchProfile === 'sub'
                        ? 'bg-sky-500/20 border-sky-500/50 text-sky-300'
                        : 'bg-slate-950 border-slate-800 text-slate-400 hover:border-slate-700'
                    }`}
                  >
                    Secundário (102)
                  </button>
                </div>
              </div>

              <div className="pt-3 border-t border-slate-800 flex items-center justify-end gap-3">
                <button
                  type="button"
                  onClick={() => setIsBatchOpen(false)}
                  className="px-4 py-2 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-300 text-xs font-medium transition"
                >
                  Voltar
                </button>
                <button
                  type="submit"
                  disabled={isSavingBatch}
                  className="px-5 py-2 rounded-lg bg-sky-600 hover:bg-sky-500 text-white text-xs font-bold shadow-md shadow-sky-950 transition flex items-center gap-2 disabled:opacity-50"
                >
                  {isSavingBatch ? (
                    <Loader2 className="h-4 w-4 animate-spin" />
                  ) : (
                    <Check className="h-4 w-4" />
                  )}
                  <span>Salvar {selectedIps.size} Câmeras</span>
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
};
