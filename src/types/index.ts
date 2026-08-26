export type StreamState = 'online' | 'offline' | 'connecting' | 'error';

export type DeviceType =
  | 'ip_camera'
  | 'nvr'
  | 'intercom'
  | 'traffic_lpr'
  | 'ptz'
  | 'thermal'
  | 'other';

export interface Camera {
  id: string;
  name: string;
  host: string;
  username: string;
  rtsp_port: number;
  rtsp_url: string;
  stream_profile: string; // 'main' | 'sub' | 'custom'
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateCameraInput {
  name: string;
  host: string;
  username: string;
  password?: string;
  rtsp_port?: number;
  rtsp_url?: string;
  stream_profile?: string;
  enabled?: boolean;
}

export interface UpdateCameraInput {
  id: string;
  name?: string;
  host?: string;
  username?: string;
  password?: string;
  rtsp_port?: number;
  rtsp_url?: string;
  stream_profile?: string;
  enabled?: boolean;
}

export interface DiscoveredDevice {
  ip: string;
  name: string;
  hardware_model: string;
  brand: string;
  device_type: DeviceType;
  device_type_label: string;
  xaddrs: string;
  rtsp_port: number;
  is_already_added: bool_or_boolean;
}

type bool_or_boolean = boolean;

export interface BatchDeviceItem {
  name: string;
  host: string;
  rtsp_port: number;
  custom_rtsp_url?: string;
}

export interface BatchCreateCamerasInput {
  devices: BatchDeviceItem[];
  username: string;
  password?: string;
  stream_profile: string;
}

export interface CameraConnectionTestResult {
  success: boolean;
  message: string;
  codec?: string;
  resolution?: string;
  fps?: number;
  bitrate?: string;
  latency_ms?: number;
}

export interface CameraStreamStatus {
  camera_id: string;
  state: StreamState;
  fps: number;
  bitrate_kbps: number;
  resolution: string;
  codec: string;
  reconnect_attempts: number;
  last_frame_time?: string;
  error_message?: string;
  stream_url: string;
}

export interface LogEntry {
  timestamp: string;
  level: string;
  target: string;
  message: string;
}

export interface AppConfig {
  app_name: string;
  video_server_port: number;
  database_path: string;
  auto_reconnect_interval_secs: number;
  default_grid_layout: number;
}
