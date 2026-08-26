import React from 'react';
import {
  LayoutDashboard,
  Radio,
  Settings,
  Shield,
} from 'lucide-react';

export type NavTab = 'dashboard' | 'live' | 'settings';

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
  const menuItems: { id: NavTab; label: string; icon: React.ReactNode; badge?: string }[] = [
    {
      id: 'dashboard',
      label: 'Instalação & Câmeras',
      icon: <LayoutDashboard className="h-5 w-5" />,
      badge: `${onlineCount}/${totalCount}`,
    },
    {
      id: 'live',
      label: 'Visualização Ao Vivo',
      icon: <Radio className="h-5 w-5" />,
    },
    {
      id: 'settings',
      label: 'Configurações',
      icon: <Settings className="h-5 w-5" />,
    },
  ];

  return (
    <aside className="w-64 bg-slate-950 border-r border-slate-800/80 flex flex-col justify-between select-none shrink-0">
      {/* Brand Header */}
      <div>
        <div className="p-5 flex items-center gap-3 border-b border-slate-800/60">
          <div className="h-10 w-10 rounded-xl bg-gradient-to-tr from-sky-600 to-cyan-400 flex items-center justify-center text-slate-950 font-bold shadow-lg shadow-sky-500/20">
            <Shield className="h-6 w-6 text-white" />
          </div>
          <div>
            <h1 className="text-base font-black tracking-tight text-white flex items-center gap-1.5">
              ONLIVIEW
              <span className="text-[10px] uppercase font-bold px-1.5 py-0.2 rounded bg-sky-500/20 text-sky-400 border border-sky-500/30">
                FAST
              </span>
            </h1>
            <p className="text-[11px] text-slate-400 font-mono">Comissionamento CFTV</p>
          </div>
        </div>

        {/* Navigation Tabs */}
        <nav className="p-3 space-y-1">
          {menuItems.map((item) => {
            const isActive = currentTab === item.id;
            return (
              <button
                key={item.id}
                onClick={() => onSelectTab(item.id)}
                className={`w-full flex items-center justify-between px-3.5 py-2.5 rounded-lg text-sm font-medium transition-all ${
                  isActive
                    ? 'bg-sky-600 text-white font-semibold shadow-md shadow-sky-950'
                    : 'text-slate-400 hover:text-slate-100 hover:bg-slate-900/80'
                }`}
              >
                <div className="flex items-center gap-3">
                  <span className={isActive ? 'text-white' : 'text-slate-400'}>{item.icon}</span>
                  <span>{item.label}</span>
                </div>
                {item.badge && (
                  <span
                    className={`text-[11px] font-mono px-2 py-0.5 rounded-full ${
                      isActive
                        ? 'bg-sky-800/80 text-sky-100'
                        : 'bg-slate-900 text-slate-400 border border-slate-800'
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
      <div className="p-4 border-t border-slate-800/60 bg-slate-950/40 text-[11px] text-slate-400 space-y-1 font-mono">
        <div className="flex justify-between">
          <span>Engine:</span>
          <span className="text-slate-300 font-bold">FFmpeg Low-Delay</span>
        </div>
        <div className="flex justify-between">
          <span>Decodificação:</span>
          <span className="text-emerald-400 font-bold">Hardware VA-API</span>
        </div>
      </div>
    </aside>
  );
};
