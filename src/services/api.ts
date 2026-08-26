import { invoke } from '@tauri-apps/api/core';
import type {
  Camera,
  CreateCameraInput,
  UpdateCameraInput,
  DiscoveredDevice,
  NetworkInterfaceInfo,
  BatchCreateCamerasInput,
  CameraConnectionTestResult,
  CameraStreamStatus,
  LogEntry,
  AppConfig,
  QuickViewConnectInput,
  QuickViewSessionInfo,
  QuickViewSetDeviceNameInput,
  QuickViewSetOsdInput,
} from '@/types';

export const api = {
  async getNetworkInterfaces(): Promise<NetworkInterfaceInfo[]> {
    return await invoke('get_network_interfaces');
  },

  async getCameras(): Promise<Camera[]> {
    return await invoke('get_cameras');
  },

  async getCamera(id: string): Promise<Camera | null> {
    return await invoke('get_camera', { id });
  },

  async createCamera(input: CreateCameraInput): Promise<Camera> {
    return await invoke('create_camera', { input });
  },

  async createCamerasBatch(input: BatchCreateCamerasInput): Promise<Camera[]> {
    return await invoke('create_cameras_batch', { input });
  },

  async discoverDevices(interfaceName?: string): Promise<DiscoveredDevice[]> {
    return await invoke('discover_devices', { interfaceName });
  },

  async updateCamera(input: UpdateCameraInput): Promise<Camera> {
    return await invoke('update_camera', { input });
  },

  async deleteCamera(id: string): Promise<void> {
    return await invoke('delete_camera', { id });
  },

  async deleteCamerasBatch(ids: string[]): Promise<number> {
    return await invoke('delete_cameras_batch', { ids });
  },

  async deleteAllCameras(): Promise<number> {
    return await invoke('delete_all_cameras');
  },

  async testCameraConnection(input: CreateCameraInput): Promise<CameraConnectionTestResult> {
    return await invoke('test_camera_connection', { input });
  },

  async testExistingCamera(id: string): Promise<CameraConnectionTestResult> {
    return await invoke('test_existing_camera', { id });
  },

  async startStream(cameraId: string): Promise<void> {
    return await invoke('start_stream', { cameraId });
  },

  async stopStream(cameraId: string): Promise<void> {
    return await invoke('stop_stream', { cameraId });
  },

  async getStreamStatus(cameraId: string): Promise<CameraStreamStatus | null> {
    return await invoke('get_stream_status', { cameraId });
  },

  async getAllStreamStatuses(): Promise<CameraStreamStatus[]> {
    return await invoke('get_all_stream_statuses');
  },

  async getLogs(): Promise<LogEntry[]> {
    return await invoke('get_logs');
  },

  async clearLogs(): Promise<void> {
    return await invoke('clear_logs');
  },

  async getAppConfig(): Promise<AppConfig> {
    return await invoke('get_app_config');
  },

  async quickViewConnect(input: QuickViewConnectInput): Promise<QuickViewSessionInfo> {
    return await invoke('quick_view_connect', { input });
  },

  async quickViewDisconnect(ip: string): Promise<void> {
    return await invoke('quick_view_disconnect', { ip });
  },

  async quickViewSetDeviceName(input: QuickViewSetDeviceNameInput): Promise<void> {
    return await invoke('quick_view_set_device_name', { input });
  },

  async quickViewSetOsd(input: QuickViewSetOsdInput): Promise<void> {
    return await invoke('quick_view_set_osd', { input });
  },

  async startDevicePreview(input: QuickViewConnectInput): Promise<string> {
    return await invoke('start_device_preview', { input });
  },

  async stopDevicePreview(ip: string): Promise<void> {
    return await invoke('stop_device_preview', { ip });
  },
};
