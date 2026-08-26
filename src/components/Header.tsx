import React from 'react';
import { RefreshCw, Sun, Moon } from 'lucide-react';
import { useTheme } from '@/context/ThemeContext';

interface HeaderProps {
  title: string;
  subtitle?: string;
  onRefresh?: () => void;
  isRefreshing?: boolean;
  extraActions?: React.ReactNode;
}

export const Header: React.FC<HeaderProps> = ({
  title,
  subtitle,
  onRefresh,
  isRefreshing = false,
  extraActions,
}) => {
  const { theme, toggleTheme } = useTheme();

  return (
    <header className="h-14 bg-white dark:bg-slate-900 border-b border-slate-200 dark:border-slate-800 px-6 flex items-center justify-between transition-colors shrink-0 shadow-sm">
      <div>
        <h2 className="text-base font-bold text-slate-800 dark:text-white tracking-tight flex items-center gap-2">
          {title}
        </h2>
        {subtitle && <p className="text-xs text-slate-500 dark:text-slate-400">{subtitle}</p>}
      </div>

      <div className="flex items-center gap-2.5">
        {extraActions}

        {/* Theme Toggle Button */}
        <button
          onClick={toggleTheme}
          className="px-2.5 py-1.5 rounded-lg bg-slate-100 dark:bg-slate-800 hover:bg-slate-200 dark:hover:bg-slate-700 text-slate-700 dark:text-slate-200 transition border border-slate-200 dark:border-slate-700 text-xs font-semibold flex items-center gap-1.5 shadow-sm"
          title={theme === 'dark' ? 'Alternar para Tema Claro (HiTools White)' : 'Alternar para Tema Escuro (Dark)'}
        >
          {theme === 'dark' ? (
            <>
              <Sun className="h-3.5 w-3.5 text-amber-400" />
              <span>Tema Claro</span>
            </>
          ) : (
            <>
              <Moon className="h-3.5 w-3.5 text-sky-600" />
              <span>Tema Escuro</span>
            </>
          )}
        </button>

        {onRefresh && (
          <button
            onClick={onRefresh}
            disabled={isRefreshing}
            className="p-1.5 rounded-lg bg-slate-100 dark:bg-slate-800 hover:bg-slate-200 dark:hover:bg-slate-700 text-slate-700 dark:text-slate-300 transition border border-slate-200 dark:border-slate-700 disabled:opacity-50 shadow-sm"
            title="Atualizar status"
          >
            <RefreshCw className={`h-4 w-4 ${isRefreshing ? 'animate-spin text-sky-500' : ''}`} />
          </button>
        )}
      </div>
    </header>
  );
};
