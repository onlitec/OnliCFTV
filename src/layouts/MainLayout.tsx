import React from 'react';
import { Sidebar, NavTab } from '@/components/Sidebar';
import { Header } from '@/components/Header';

interface MainLayoutProps {
  currentTab: NavTab;
  onSelectTab: (tab: NavTab) => void;
  onlineCount: number;
  totalCount: number;
  title: string;
  subtitle?: string;
  onRefresh?: () => void;
  isRefreshing?: boolean;
  extraHeaderActions?: React.ReactNode;
  children: React.ReactNode;
}

export const MainLayout: React.FC<MainLayoutProps> = ({
  currentTab,
  onSelectTab,
  onlineCount,
  totalCount,
  title,
  subtitle,
  onRefresh,
  isRefreshing,
  extraHeaderActions,
  children,
}) => {
  return (
    <div className="flex h-screen w-screen bg-slate-100 dark:bg-slate-950 text-slate-900 dark:text-slate-100 overflow-hidden font-sans transition-colors">
      <Sidebar
        currentTab={currentTab}
        onSelectTab={onSelectTab}
        onlineCount={onlineCount}
        totalCount={totalCount}
      />

      <div className="flex-1 flex flex-col min-w-0 overflow-hidden">
        <Header
          title={title}
          subtitle={subtitle}
          onRefresh={onRefresh}
          isRefreshing={isRefreshing}
          extraActions={extraHeaderActions}
        />

        <main className="flex-1 overflow-hidden relative bg-slate-100/70 dark:bg-slate-950/60">
          {children}
        </main>
      </div>
    </div>
  );
};
