import React, { useState, useEffect } from 'react';
import { Settings, Database, Shield, Cpu } from 'lucide-react';
import type { AppConfig } from '@/types';
import { api } from '@/services/api';

export const SettingsPage: React.FC = () => {
  const [config, setConfig] = useState<AppConfig | null>(null);

  useEffect(() => {
    api.getAppConfig().then(setConfig).catch(console.error);
  }, []);

  return (
    <div className="p-6 space-y-6 max-h-full overflow-y-auto select-none">
      <div>
        <h3 className="text-xl font-bold text-white tracking-tight flex items-center gap-2">
          <Settings className="h-5 w-5 text-sky-400" />
          Configurações do Sistema
        </h3>
        <p className="text-xs text-slate-400 mt-1">
          Parâmetros operacionais do motor de vídeo, armazenamento local e rede.
        </p>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        {/* Storage and Database */}
        <div className="bg-slate-900/90 border border-slate-800 rounded-xl p-5 shadow space-y-4">
          <div className="flex items-center gap-2 text-sm font-bold text-white border-b border-slate-800 pb-3">
            <Database className="h-4 w-4 text-sky-400" />
            <span>Banco de Dados Local (SQLite)</span>
          </div>

          <div className="space-y-3 text-xs">
            <div>
              <span className="text-slate-400 block mb-1">Caminho do Banco de Dados:</span>
              <div className="p-2.5 bg-slate-950 rounded-lg border border-slate-800 font-mono text-slate-200 text-[11px] break-all">
                {config?.database_path || '~/.config/onliview/onliview.db'}
              </div>
            </div>
            <div>
              <span className="text-slate-400 block mb-1">Criptografia de Credenciais:</span>
              <div className="p-2.5 bg-slate-950 rounded-lg border border-slate-800 text-emerald-400 font-mono text-[11px] flex items-center gap-2">
                <Shield className="h-3.5 w-3.5" />
                <span>AES-256-GCM (Chave derivada por máquina)</span>
              </div>
            </div>
          </div>
        </div>

        {/* Video Engine & Network */}
        <div className="bg-slate-900/90 border border-slate-800 rounded-xl p-5 shadow space-y-4">
          <div className="flex items-center gap-2 text-sm font-bold text-white border-b border-slate-800 pb-3">
            <Cpu className="h-4 w-4 text-sky-400" />
            <span>Motor de Vídeo e Streaming</span>
          </div>

          <div className="space-y-3 text-xs">
            <div>
              <span className="text-slate-400 block mb-1">Porta do Servidor Local MJPEG:</span>
              <div className="p-2.5 bg-slate-950 rounded-lg border border-slate-800 font-mono text-slate-200 text-[11px]">
                {config?.video_server_port || 18554}
              </div>
            </div>
            <div>
              <span className="text-slate-400 block mb-1">Intervalo de Reconexão Automática:</span>
              <div className="p-2.5 bg-slate-950 rounded-lg border border-slate-800 font-mono text-slate-200 text-[11px]">
                {config?.auto_reconnect_interval_secs || 5} segundos
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
