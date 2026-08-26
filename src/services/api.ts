import { invoke } from '@tauri-apps/api/core';
import type {
  Camera,
  CreateCameraInput,
  UpdateCameraInput,
  CameraConnectionTestResult,
  CameraStreamStatus,
  LogEntry,
  AppConfig,
} from '@/types';

export const api = {
  async getCameras(): Promise<Camera[]> {
    return await invoke('get_cameras');
  },

  async getCamera(id: string): Promise<Camera | null> {
    return await invoke('get_camera', { id });
  },

  async createCamera(input: CreateCameraInput): Promise<Camera> {
    return await invoke('create_camera', { input });
  },

  async updateCamera(input: UpdateCameraInput): Promise<Camera> {
    return await invoke('update_camera', { input });
  },

  async deleteCamera(id: string): Promise<void> {
    return await invoke('delete_camera', { id });
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
};
