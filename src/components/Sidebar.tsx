import React from 'react';
import {
  LayoutDashboard,
  Camera as CameraIcon,
  Grid,
  Film,
  Bell,
  Activity,
  Settings,
  ShieldCheck,
} from 'lucide-react';

export type NavTab =
  | 'dashboard'
  | 'cameras'
  | 'live'
  | 'recordings'
  | 'events'
  | 'diagnostics'
  | 'settings';

interface SidebarProps {
  currentTab: NavTab;
  onSelectTab: (tab: NavTab) => void;
  onlineCount: number;
  totalCount: number;
}

export const Sidebar: React.FC<SidebarProps> = ({
  currentTab,
  onSelectTab,
  onlineCount,
  totalCount,
}) => {
  const navItems: { id: NavTab; label: string; icon: React.ComponentType<{ className?: string }>; badge?: string }[] = [
    { id: 'dashboard', label: 'Dashboard', icon: LayoutDashboard },
    { id: 'cameras', label: 'Câmeras', icon: CameraIcon, badge: totalCount > 0 ? `${totalCount}` : undefined },
    { id: 'live', label: 'Visualização', icon: Grid, badge: onlineCount > 0 ? `${onlineCount}` : undefined },
    { id: 'recordings', label: 'Gravações', icon: Film },
    { id: 'events', label: 'Eventos', icon: Bell },
    { id: 'diagnostics', label: 'Diagnóstico', icon: Activity },
    { id: 'settings', label: 'Configurações', icon: Settings },
  ];

  return (
    <aside className="w-64 bg-slate-900/95 border-r border-slate-800/80 flex flex-col justify-between select-none">
      <div>
        {/* Brand Header */}
        <div className="h-16 flex items-center gap-3 px-5 border-b border-slate-800/80 bg-slate-950/40">
          <div className="h-9 w-9 rounded-lg bg-sky-500/20 border border-sky-500/30 flex items-center justify-center text-sky-400 font-bold shadow-inner">
            <CameraIcon className="h-5 w-5" />
          </div>
          <div>
            <h1 className="font-bold text-base tracking-wide text-white flex items-center gap-1.5">
              ONLIVIEW
              <span className="text-[10px] px-1.5 py-0.5 rounded bg-sky-500/20 text-sky-400 font-semibold border border-sky-500/30">
                VMS
              </span>
            </h1>
            <p className="text-xs text-slate-400">Onlitec Security Suite</p>
          </div>
        </div>

        {/* Navigation Menu */}
        <nav className="p-3 space-y-1.5 mt-2">
          {navItems.map((item) => {
            const Icon = item.icon;
            const isActive = currentTab === item.id;
            return (
              <button
                key={item.id}
                onClick={() => onSelectTab(item.id)}
                className={`w-full flex items-center justify-between px-3.5 py-2.5 rounded-lg text-sm font-medium transition-all ${
                  isActive
                    ? 'bg-sky-500/15 text-sky-400 border border-sky-500/30 shadow-sm'
                    : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800/60'
                }`}
              >
                <div className="flex items-center gap-3">
                  <Icon className={`h-4 w-4 ${isActive ? 'text-sky-400' : 'text-slate-400'}`} />
                  <span>{item.label}</span>
                </div>
                {item.badge && (
                  <span
                    className={`text-xs px-2 py-0.5 rounded-full font-mono ${
                      isActive
                        ? 'bg-sky-500/30 text-sky-300'
                        : 'bg-slate-800 text-slate-400'
                    }`}
                  >
                    {item.badge}
                  </span>
                )}
              </button>
            );
          })}
        </nav>
      </div>

      {/* System Status Footer */}
      <div className="p-4 border-t border-slate-800/80 bg-slate-950/40">
        <div className="flex items-center gap-2 text-xs text-slate-400 mb-2">
          <ShieldCheck className="h-4 w-4 text-emerald-400" />
          <span className="font-semibold text-slate-300">Motor de Vídeo Ativo</span>
        </div>
        <div className="flex justify-between items-center text-[11px] text-slate-400">
          <span>Câmeras Conectadas:</span>
          <span className="font-mono text-emerald-400 font-bold">{onlineCount} / {totalCount}</span>
        </div>
      </div>
    </aside>
  );
};
