import React, { useState, useEffect } from 'react';
import {
  Activity,
  RefreshCw,
  Trash2,
  Search,
  Terminal,
} from 'lucide-react';
import type { Camera, CameraStreamStatus, LogEntry } from '@/types';
import { api } from '@/services/api';

interface DiagnosticsProps {
  cameras: Camera[];
  streamStatuses: Record<string, CameraStreamStatus>;
  serverPort: number;
}

export const DiagnosticsPage: React.FC<DiagnosticsProps> = ({
  cameras,
  streamStatuses,
  serverPort,
}) => {
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [filterLevel, setFilterLevel] = useState<string>('ALL');
  const [searchQuery, setSearchQuery] = useState<string>('');
  const [isRefreshing, setIsRefreshing] = useState(false);

  const loadLogs = async () => {
    setIsRefreshing(true);
    try {
      const entries = await api.getLogs();
      setLogs(entries);
    } catch (e) {
      console.error(e);
    } finally {
      setIsRefreshing(false);
    }
  };

  useEffect(() => {
    loadLogs();
    const interval = setInterval(loadLogs, 3000);
    return () => clearInterval(interval);
  }, []);

  const handleClearLogs = async () => {
    await api.clearLogs();
    setLogs([]);
  };

  const filteredLogs = logs.filter((log) => {
    if (filterLevel !== 'ALL' && log.level !== filterLevel) return false;
    if (searchQuery) {
      const q = searchQuery.toLowerCase();
      return (
        log.message.toLowerCase().includes(q) ||
        log.target.toLowerCase().includes(q) ||
        log.level.toLowerCase().includes(q)
      );
    }
    return true;
  });

  return (
    <div className="p-6 space-y-6 max-h-full overflow-y-auto select-none">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-xl font-bold text-white tracking-tight flex items-center gap-2">
            <Activity className="h-5 w-5 text-sky-400" />
            Diagnóstico e Telemetria Técnica
          </h3>
          <p className="text-xs text-slate-400 mt-1">
            Inspeção em tempo real de conexões RTSP, codecs, taxas de quadros e logs estruturados.
          </p>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={loadLogs}
            disabled={isRefreshing}
            className="p-2 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-300 border border-slate-700 text-xs flex items-center gap-1.5 transition"
            title="Atualizar Logs"
          >
            <RefreshCw className={`h-4 w-4 ${isRefreshing ? 'animate-spin text-sky-400' : ''}`} />
          </button>
          <button
            onClick={handleClearLogs}
            className="px-3 py-1.5 rounded-lg bg-slate-800 hover:bg-rose-500/20 text-slate-300 hover:text-rose-300 border border-slate-700 hover:border-rose-500/30 text-xs font-semibold flex items-center gap-1.5 transition"
          >
            <Trash2 className="h-4 w-4" />
            <span>Limpar Logs</span>
          </button>
        </div>
      </div>

      {/* Camera Telemetry Table */}
      <div className="bg-slate-900/90 border border-slate-800 rounded-xl overflow-hidden shadow">
        <div className="p-4 border-b border-slate-800 bg-slate-950/40 flex items-center justify-between">
          <span className="text-xs font-bold text-white uppercase tracking-wider">
            Telemetria de Streams RTSP
          </span>
          <span className="text-xs text-slate-400 font-mono">
            Servidor Local MJPEG: 127.0.0.1:{serverPort}
          </span>
        </div>

        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs font-mono">
            <thead className="bg-slate-950/80 text-slate-400 uppercase text-[10px] tracking-wider border-b border-slate-800">
              <tr>
                <th className="px-4 py-3">Câmera</th>
                <th className="px-4 py-3">IP : Porta</th>
                <th className="px-4 py-3">Estado</th>
                <th className="px-4 py-3">FPS</th>
                <th className="px-4 py-3">Bitrate</th>
                <th className="px-4 py-3">Reconexões</th>
                <th className="px-4 py-3">Último Frame</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-800/60">
              {cameras.length === 0 ? (
                <tr>
                  <td colSpan={7} className="px-4 py-6 text-center text-slate-500">
                    Nenhuma câmera configurada
                  </td>
                </tr>
              ) : (
                cameras.map((cam) => {
                  const st = streamStatuses[cam.id];
                  const state = st?.state || 'offline';
                  return (
                    <tr key={cam.id} className="hover:bg-slate-800/40">
                      <td className="px-4 py-3 font-semibold text-white font-sans">{cam.name}</td>
                      <td className="px-4 py-3 text-slate-300">{cam.host}:{cam.rtsp_port}</td>
                      <td className="px-4 py-3">
                        <span
                          className={`px-2 py-0.5 rounded text-[10px] font-bold uppercase ${
                            state === 'online'
                              ? 'bg-emerald-500/20 text-emerald-400'
                              : state === 'connecting'
                              ? 'bg-amber-500/20 text-amber-400'
                              : 'bg-rose-500/20 text-rose-400'
                          }`}
                        >
                          {state}
                        </span>
                      </td>
                      <td className="px-4 py-3 text-emerald-400 font-bold">{st?.fps || 0}</td>
                      <td className="px-4 py-3 text-slate-300">{st?.bitrate_kbps || 0} kbps</td>
                      <td className="px-4 py-3 text-slate-400">{st?.reconnect_attempts || 0}</td>
                      <td className="px-4 py-3 text-slate-400 truncate max-w-xs">
                        {st?.last_frame_time ? new Date(st.last_frame_time).toLocaleTimeString() : '---'}
                      </td>
                    </tr>
                  );
                })
              )}
            </tbody>
          </table>
        </div>
      </div>

      {/* Logs Console */}
      <div className="bg-slate-900/90 border border-slate-800 rounded-xl overflow-hidden shadow">
        <div className="p-4 border-b border-slate-800 bg-slate-950/40 flex flex-wrap items-center justify-between gap-3">
          <div className="flex items-center gap-2">
            <Terminal className="h-4 w-4 text-sky-400" />
            <span className="text-xs font-bold text-white uppercase tracking-wider">
              Logs Técnicos do Sistema (Sanitizados)
            </span>
          </div>

          <div className="flex items-center gap-2">
            {/* Search */}
            <div className="relative">
              <input
                type="text"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder="Buscar nos logs..."
                className="bg-slate-950 border border-slate-800 rounded-lg pl-8 pr-3 py-1.5 text-xs text-white placeholder-slate-500 focus:outline-none focus:border-sky-500"
              />
              <Search className="h-3.5 w-3.5 text-slate-500 absolute left-2.5 top-1/2 -translate-y-1/2" />
            </div>

            {/* Level Filter */}
            <select
              value={filterLevel}
              onChange={(e) => setFilterLevel(e.target.value)}
              className="bg-slate-950 border border-slate-800 rounded-lg px-2.5 py-1.5 text-xs text-white focus:outline-none focus:border-sky-500 font-mono"
            >
              <option value="ALL">TODOS OS NÍVEIS</option>
              <option value="INFO">INFO</option>
              <option value="WARN">WARN</option>
              <option value="ERROR">ERROR</option>
              <option value="DEBUG">DEBUG</option>
            </select>
          </div>
        </div>

        {/* Logs Output List */}
        <div className="p-4 bg-slate-950 font-mono text-xs max-h-80 overflow-y-auto space-y-1.5">
          {filteredLogs.length === 0 ? (
            <div className="text-slate-500 text-center py-6">Nenhum log registrado no momento.</div>
          ) : (
            filteredLogs.map((log, i) => {
              const isError = log.level === 'ERROR';
              const isWarn = log.level === 'WARN';

              return (
                <div
                  key={i}
                  className="flex items-start gap-2 hover:bg-slate-900/60 p-1 rounded transition"
                >
                  <span className="text-slate-500 shrink-0 select-none">
                    {new Date(log.timestamp).toLocaleTimeString()}
                  </span>
                  <span
                    className={`px-1.5 py-0.2 rounded text-[10px] font-bold shrink-0 ${
                      isError
                        ? 'bg-rose-500/20 text-rose-400 border border-rose-500/30'
                        : isWarn
                        ? 'bg-amber-500/20 text-amber-400 border border-amber-500/30'
                        : 'bg-sky-500/20 text-sky-400 border border-sky-500/30'
                    }`}
                  >
                    {log.level}
                  </span>
                  <span className="text-slate-400 shrink-0">[{log.target}]</span>
                  <span className="text-slate-200 break-all">{log.message}</span>
                </div>
              );
            })
          )}
        </div>
      </div>
    </div>
  );
};
