import React from 'react';
import { Bell, ShieldAlert } from 'lucide-react';

export const EventsPage: React.FC = () => {
  return (
    <div className="p-6 space-y-6 max-h-full overflow-y-auto select-none">
      <div>
        <h3 className="text-xl font-bold text-white tracking-tight flex items-center gap-2">
          <Bell className="h-5 w-5 text-sky-400" />
          Central de Eventos e Notificações
        </h3>
        <p className="text-xs text-slate-400 mt-1">
          Registro de perda de sinal, reconexões, alarmes e detecção de movimento.
        </p>
      </div>

      <div className="bg-slate-900/60 border border-dashed border-slate-800 rounded-xl p-12 text-center">
        <div className="h-12 w-12 rounded-full bg-slate-800 flex items-center justify-center mx-auto text-slate-400 mb-3">
          <ShieldAlert className="h-6 w-6 text-sky-400" />
        </div>
        <h4 className="text-base font-bold text-white mb-1">Central de Alarmes</h4>
        <p className="text-xs text-slate-400 max-w-sm mx-auto">
          Notificações e eventos de stream em tempo real integrados com o motor de análise.
        </p>
      </div>
    </div>
  );
};
