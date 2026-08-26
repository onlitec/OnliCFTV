import React from 'react';
import { Film, Clock } from 'lucide-react';

export const RecordingsPage: React.FC = () => {
  return (
    <div className="p-6 space-y-6 max-h-full overflow-y-auto select-none">
      <div>
        <h3 className="text-xl font-bold text-white tracking-tight flex items-center gap-2">
          <Film className="h-5 w-5 text-sky-400" />
          Gravações e Reprodução (Playback)
        </h3>
        <p className="text-xs text-slate-400 mt-1">
          Histórico de gravações contínuas e por detecção de eventos no armazenamento local ou NVR.
        </p>
      </div>

      <div className="bg-slate-900/60 border border-dashed border-slate-800 rounded-xl p-12 text-center">
        <div className="h-12 w-12 rounded-full bg-slate-800 flex items-center justify-center mx-auto text-slate-400 mb-3">
          <Clock className="h-6 w-6 text-sky-400" />
        </div>
        <h4 className="text-base font-bold text-white mb-1">Módulo de Gravação Planejado</h4>
        <p className="text-xs text-slate-400 max-w-sm mx-auto">
          O módulo de gravação em disco local e busca em NVRs Hikvision (ISAPI / ONVIF Replay) estará disponível nas próximas sprints.
        </p>
      </div>
    </div>
  );
};
