import React, { useState, useEffect, useCallback } from 'react';
import { MainLayout } from '@/layouts/MainLayout';
import type { NavTab } from '@/components/Sidebar';
import { DashboardPage } from '@/pages/DashboardPage';
import { CameraList } from '@/cameras/CameraList';
import { CameraModal } from '@/cameras/CameraModal';
import { LiveView } from '@/video/LiveView';
import { DiagnosticsPage } from '@/diagnostics/DiagnosticsPage';
import { SettingsPage } from '@/settings/SettingsPage';
import { RecordingsPage } from '@/pages/RecordingsPage';
import { EventsPage } from '@/pages/EventsPage';

import type { Camera, CameraStreamStatus, AppConfig } from '@/types';
import { api } from '@/services/api';

export const App: React.FC = () => {
  const [currentTab, setCurrentTab] = useState<NavTab>('dashboard');
  const [cameras, setCameras] = useState<Camera[]>([]);
  const [streamStatuses, setStreamStatuses] = useState<Record<string, CameraStreamStatus>>({});
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [cameraToEdit, setCameraToEdit] = useState<Camera | null>(null);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [appConfig, setAppConfig] = useState<AppConfig | null>(null);

  // Load cameras & config from SQLite database
  const loadData = useCallback(async () => {
    setIsRefreshing(true);
    try {
      const [cams, cfg, statuses] = await Promise.all([
        api.getCameras(),
        api.getAppConfig(),
        api.getAllStreamStatuses(),
      ]);
      setCameras(cams);
      setAppConfig(cfg);

      const statusMap: Record<string, CameraStreamStatus> = {};
      for (const st of statuses) {
        statusMap[st.camera_id] = st;
      }
      setStreamStatuses(statusMap);
    } catch (e) {
      console.error('Failed to load cameras or statuses:', e);
    } finally {
      setIsRefreshing(false);
    }
  }, []);

  useEffect(() => {
    loadData();
    // Poll telemetry stream statuses every 2 seconds
    const interval = setInterval(async () => {
      try {
        const statuses = await api.getAllStreamStatuses();
        const statusMap: Record<string, CameraStreamStatus> = {};
        for (const st of statuses) {
          statusMap[st.camera_id] = st;
        }
        setStreamStatuses(statusMap);
      } catch (e) {
        // ignore polling errors
      }
    }, 2000);

    return () => clearInterval(interval);
  }, [loadData]);

  const handleAddCamera = () => {
    setCameraToEdit(null);
    setIsModalOpen(true);
  };

  const handleEditCamera = (cam: Camera) => {
    setCameraToEdit(cam);
    setIsModalOpen(true);
  };

  const handleDeleteCamera = async (id: string) => {
    try {
      await api.deleteCamera(id);
      await loadData();
    } catch (e) {
      console.error('Failed to delete camera:', e);
    }
  };

  const handleStartStream = async (id: string) => {
    try {
      await api.startStream(id);
      const st = await api.getStreamStatus(id);
      if (st) {
        setStreamStatuses((prev) => ({ ...prev, [id]: st }));
      }
    } catch (e) {
      console.error('Failed to start stream:', e);
    }
  };

  const handleStopStream = async (id: string) => {
    try {
      await api.stopStream(id);
      setStreamStatuses((prev) => {
        const next = { ...prev };
        delete next[id];
        return next;
      });
    } catch (e) {
      console.error('Failed to stop stream:', e);
    }
  };

  const handleStartAll = async () => {
    for (const cam of cameras) {
      if (cam.enabled) {
        handleStartStream(cam.id);
      }
    }
  };

  const handleStopAll = async () => {
    for (const cam of cameras) {
      handleStopStream(cam.id);
    }
  };

  const onlineCount = Object.values(streamStatuses).filter((s) => s.state === 'online').length;
  const totalCount = cameras.length;
  const serverPort = appConfig?.video_server_port || 18554;

  const tabTitles: Record<NavTab, { title: string; subtitle: string }> = {
    dashboard: {
      title: 'Painel Geral de Monitoramento',
      subtitle: 'Visão executiva e telemetria de dispositivos',
    },
    cameras: {
      title: 'Cadastro e Gerenciamento de Câmeras',
      subtitle: 'Configuração de streams RTSP, credenciais e perfis de vídeo',
    },
    live: {
      title: 'Mosaico de Visualização Ao Vivo',
      subtitle: 'Monitoramento multitelas de canais RTSP',
    },
    recordings: {
      title: 'Gravações e Playback',
      subtitle: 'Histórico de vídeo e reprodução',
    },
    events: {
      title: 'Central de Eventos',
      subtitle: 'Logs de alarmes e conectividade',
    },
    diagnostics: {
      title: 'Diagnóstico e Telemetria de Streams',
      subtitle: 'Taxas de quadros, bitrate, latência e logs estruturados',
    },
    settings: {
      title: 'Configurações do OnliView',
      subtitle: 'Banco de dados SQLite, portas e parâmetros locais',
    },
  };

  return (
    <>
      <MainLayout
        currentTab={currentTab}
        onSelectTab={setCurrentTab}
        onlineCount={onlineCount}
        totalCount={totalCount}
        title={tabTitles[currentTab].title}
        subtitle={tabTitles[currentTab].subtitle}
        onRefresh={loadData}
        isRefreshing={isRefreshing}
      >
        {currentTab === 'dashboard' && (
          <DashboardPage
            cameras={cameras}
            streamStatuses={streamStatuses}
            onNavigateTo={setCurrentTab}
            onStartStream={handleStartStream}
          />
        )}

        {currentTab === 'cameras' && (
          <CameraList
            cameras={cameras}
            streamStatuses={streamStatuses}
            onAddCamera={handleAddCamera}
            onEditCamera={handleEditCamera}
            onDeleteCamera={handleDeleteCamera}
            onStartStream={handleStartStream}
            onStopStream={handleStopStream}
            onViewLive={() => setCurrentTab('live')}
          />
        )}

        {currentTab === 'live' && (
          <LiveView
            cameras={cameras}
            streamStatuses={streamStatuses}
            onReconnect={handleStartStream}
            onStartAll={handleStartAll}
            onStopAll={handleStopAll}
            serverPort={serverPort}
          />
        )}

        {currentTab === 'recordings' && <RecordingsPage />}
        {currentTab === 'events' && <EventsPage />}

        {currentTab === 'diagnostics' && (
          <DiagnosticsPage
            cameras={cameras}
            streamStatuses={streamStatuses}
            serverPort={serverPort}
          />
        )}

        {currentTab === 'settings' && <SettingsPage />}
      </MainLayout>

      <CameraModal
        isOpen={isModalOpen}
        onClose={() => setIsModalOpen(false)}
        onSaved={loadData}
        cameraToEdit={cameraToEdit}
      />
    </>
  );
};
