import React, { useState, useEffect, useCallback } from 'react';
import { MainLayout } from '@/layouts/MainLayout';
import type { NavTab } from '@/components/Sidebar';
import { DashboardPage } from '@/pages/DashboardPage';
import { CameraModal } from '@/cameras/CameraModal';
import { LiveView } from '@/video/LiveView';
import { SettingsPage } from '@/settings/SettingsPage';

import type {
  Camera,
  CameraStreamStatus,
  AppConfig,
  DiscoveredDevice,
  CreateCameraInput,
} from '@/types';
import { api } from '@/services/api';

export const App: React.FC = () => {
  const [currentTab, setCurrentTab] = useState<NavTab>('dashboard');
  const [cameras, setCameras] = useState<Camera[]>([]);
  const [streamStatuses, setStreamStatuses] = useState<Record<string, CameraStreamStatus>>({});
  const [discoveredDevices, setDiscoveredDevices] = useState<DiscoveredDevice[]>([]);
  const [isScanning, setIsScanning] = useState(false);
  const [selectedLiveCameraId, setSelectedLiveCameraId] = useState<string | null>(null);

  // Modals state
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [cameraToEdit, setCameraToEdit] = useState<Camera | null>(null);
  const [prefillCameraData, setPrefillCameraData] = useState<CreateCameraInput | null>(null);

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

  // Background auto-scan for network devices
  const runDiscoveryScan = useCallback(async () => {
    setIsScanning(true);
    try {
      const found = await api.discoverDevices();
      setDiscoveredDevices(found);
    } catch (e) {
      console.error('Auto discovery scan error:', e);
    } finally {
      setIsScanning(false);
    }
  }, []);

  useEffect(() => {
    loadData();
    runDiscoveryScan();

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
  }, [loadData, runDiscoveryScan]);

  const handleAddCamera = () => {
    setCameraToEdit(null);
    setPrefillCameraData(null);
    setIsModalOpen(true);
  };

  const handleAddSingleFromDiscovery = (prefill: CreateCameraInput) => {
    setCameraToEdit(null);
    setPrefillCameraData(prefill);
    setIsModalOpen(true);
  };

  const handleEditCamera = (cam: Camera) => {
    setCameraToEdit(cam);
    setPrefillCameraData(null);
    setIsModalOpen(true);
  };

  const handleDeleteCamera = async (id: string) => {
    try {
      await api.deleteCamera(id);
      await loadData();
      runDiscoveryScan();
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

  const handleOpenLiveCamera = (cameraId: string) => {
    setSelectedLiveCameraId(cameraId || null);
    setCurrentTab('live');
  };

  const onlineCount = Object.values(streamStatuses).filter((s) => s.state === 'online').length;
  const totalCount = cameras.length;
  const serverPort = appConfig?.video_server_port || 18554;

  const tabTitles: Record<NavTab, { title: string; subtitle: string }> = {
    dashboard: {
      title: 'Comissionamento & Câmeras',
      subtitle: 'Visão de dispositivos instalados e busca inteligente na rede local',
    },
    live: {
      title: 'Visualização Ao Vivo & Enquadramento',
      subtitle: 'Monitoramento em tempo real para alinhamento de foco e OSD',
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
        onRefresh={() => {
          loadData();
          runDiscoveryScan();
        }}
        isRefreshing={isRefreshing}
      >
        {currentTab === 'dashboard' && (
          <DashboardPage
            cameras={cameras}
            streamStatuses={streamStatuses}
            discoveredDevices={discoveredDevices}
            isScanning={isScanning}
            onRefreshScan={runDiscoveryScan}
            onAddCamera={handleAddCamera}
            onEditCamera={handleEditCamera}
            onDeleteCamera={handleDeleteCamera}
            onStartStream={handleStartStream}
            onStopStream={handleStopStream}
            onOpenLiveCamera={handleOpenLiveCamera}
            onAddSingleFromDiscovery={handleAddSingleFromDiscovery}
            onDataChanged={() => {
              loadData();
              runDiscoveryScan();
            }}
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
            selectedCameraId={selectedLiveCameraId}
            onBackToDashboard={() => setCurrentTab('dashboard')}
          />
        )}

        {currentTab === 'settings' && <SettingsPage />}
      </MainLayout>

      <CameraModal
        isOpen={isModalOpen}
        onClose={() => {
          setIsModalOpen(false);
          setPrefillCameraData(null);
        }}
        onSaved={() => {
          loadData();
          runDiscoveryScan();
        }}
        cameraToEdit={cameraToEdit}
        prefillData={prefillCameraData}
      />
    </>
  );
};
