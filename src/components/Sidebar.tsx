import React, { useState } from 'react';
import {
  Search,
  Camera,
  Radio,
  Settings,
  Shield,
  Bell,
  HardDrive,
  Activity,
  ChevronLeft,
  ChevronRight,
} from 'lucide-react';

export type NavTab = 'dashboard' | 'cameras' | 'live' | 'events' | 'recordings' | 'diagnostics' | 'settings';

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
  // Starts collapsed by default on every launch (not persisted) — maximizes screen space for the
  // discovery table / Quick Viewer, which matter more than the nav on a 960x600-minimum window.
  const [isCollapsed, setIsCollapsed] = useState(true);

  const menuItems: { id: NavTab; label: string; icon: React.ReactNode; badge?: string }[] = [
    {
      id: 'dashboard',
      label: 'Descoberta & Comissionamento',
      icon: <Search className="h-4 w-4" />,
    },
    {
      id: 'cameras',
      label: 'Dispositivos Cadastrados',
      icon: <Camera className="h-4 w-4" />,
      badge: `${onlineCount}/${totalCount}`,
    },
    {
      id: 'live',
      label: 'Visualização Ao Vivo',
      icon: <Radio className="h-4 w-4" />,
    },
    {
      id: 'events',
      label: 'Eventos & Alertas',
      icon: <Bell className="h-4 w-4" />,
    },
    {
      id: 'recordings',
      label: 'Gravações & Playback',
      icon: <HardDrive className="h-4 w-4" />,
    },
    {
      id: 'diagnostics',
      label: 'Diagnóstico',
      icon: <Activity className="h-4 w-4" />,
    },
    {
      id: 'settings',
      label: 'Configurações',
      icon: <Settings className="h-4 w-4" />,
    },
  ];

  return (
    <aside
      className={`${isCollapsed ? 'w-16' : 'w-64'} bg-white dark:bg-slate-950 border-r border-slate-200 dark:border-slate-800 flex flex-col justify-between select-none shrink-0 transition-all duration-200 shadow-sm`}
    >
      {/* Brand Header */}
      <div>
        <div className={`p-4 flex items-center border-b border-slate-200 dark:border-slate-800 ${isCollapsed ? 'flex-col gap-2' : 'gap-3'}`}>
          <div className="h-9 w-9 rounded-xl bg-gradient-to-tr from-sky-600 to-cyan-400 flex items-center justify-center text-white font-bold shadow-md shadow-sky-500/20 shrink-0">
            <Shield className="h-5 w-5 text-white" />
          </div>
          {!isCollapsed && (
            <div className="min-w-0">
              <h1 className="text-sm font-black tracking-tight text-slate-800 dark:text-white flex items-center gap-1.5">
                ONLIVIEW
                <span className="text-[9px] uppercase font-bold px-1.5 py-0.2 rounded bg-sky-500/15 text-sky-600 dark:text-sky-400 border border-sky-500/30">
                  VMS
                </span>
              </h1>
              <p className="text-[10px] text-slate-500 dark:text-slate-400 font-mono">Onlitec VMS & Installer Tools</p>
            </div>
          )}
          <button
            type="button"
            onClick={() => setIsCollapsed(!isCollapsed)}
            className="ml-auto p-1 rounded-md text-slate-400 hover:text-slate-700 dark:hover:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-900 transition shrink-0"
            title={isCollapsed ? 'Expandir menu' : 'Recolher menu'}
          >
            {isCollapsed ? <ChevronRight className="h-4 w-4" /> : <ChevronLeft className="h-4 w-4" />}
          </button>
        </div>

        {/* Navigation Tabs */}
        <nav className="p-2 space-y-1">
          {menuItems.map((item) => {
            const isActive = currentTab === item.id;
            return (
              <button
                key={item.id}
                onClick={() => onSelectTab(item.id)}
                title={item.label}
                className={`w-full flex items-center rounded-lg text-xs font-semibold transition-all ${
                  isCollapsed ? 'justify-center px-2 py-2.5' : 'justify-between px-3 py-2'
                } ${
                  isActive
                    ? 'bg-sky-600 text-white font-bold shadow-md shadow-sky-500/20'
                    : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-100 hover:bg-slate-100 dark:hover:bg-slate-900'
                }`}
              >
                <div className={`flex items-center ${isCollapsed ? '' : 'gap-2.5'}`}>
                  <span className={isActive ? 'text-white' : 'text-slate-500 dark:text-slate-400'}>{item.icon}</span>
                  {!isCollapsed && <span>{item.label}</span>}
                </div>
                {!isCollapsed && item.badge && (
                  <span
                    className={`text-[10px] font-mono px-1.5 py-0.2 rounded-full ${
                      isActive
                        ? 'bg-sky-800 text-sky-100'
                        : 'bg-slate-100 dark:bg-slate-900 text-slate-600 dark:text-slate-400 border border-slate-200 dark:border-slate-800'
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

      {/* Footer Info */}
      {!isCollapsed && (
        <div className="p-3.5 border-t border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-950/60 text-[10px] text-slate-500 dark:text-slate-400 space-y-1 font-mono">
          <div className="flex justify-between">
            <span>Engine:</span>
            <span className="text-slate-700 dark:text-slate-300 font-bold">FFmpeg Low-Delay</span>
          </div>
          <div className="flex justify-between">
            <span>Decodificação:</span>
            <span className="text-emerald-600 dark:text-emerald-400 font-bold">Hardware VA-API</span>
          </div>
        </div>
      )}
    </aside>
  );
};
