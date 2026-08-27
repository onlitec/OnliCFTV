export type StreamState = 'online' | 'offline' | 'connecting' | 'error';

export type DeviceType =
  | 'ip_camera'
  | 'nvr'
  | 'dvr'
  | 'ptz'
  | 'switch'
  | 'router'
  | 'intercom'
  | 'access_control'
  | 'alarm'
  | 'server'
  | 'computer'
  | 'access_point'
  | 'thermal'
  | 'traffic_lpr'
  | 'other';

export interface NetworkInterfaceInfo {
  id: string;
  name: string;
  ip: string;
  netmask: string;
  broadcast: string;
  gateway?: string;
  mac?: string;
  is_up: boolean;
  is_default: boolean;
}

export interface DiscoveredDevice {
  id: string;
  ip: string;
  mac?: string;
  brand: string;
  hardware_model: string;
  name: string;
  device_type: DeviceType;
  device_type_label: string;
  serial_number?: string;
  firmware_version?: string;
  activation_status?: string; // 'Ativo' | 'Aguardando ativação'
  rtsp_port: number;
  http_port: number;
  sdk_port: number;
  protocols: string[];
  confidence_score: number;
  confidence_level?: string;
  evidences?: string[];
  contradictions?: string[];
  issues: string[];
  xaddrs: string;
  is_already_added: boolean;
}

export interface DiscoveryProgress {
  percentage: number;
  phase: string;
  devices_found: number;
  active_protocols: string[];
  completed_protocols: string[];
  is_running: boolean;
}

export interface Camera {
  id: string;
  name: string;
  host: string;
  username: string;
  rtsp_port: number;
  rtsp_url: string;
  stream_profile: string; // 'main' | 'sub' | 'custom'
  enabled: boolean;
  device_name?: string | null;
  osd?: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateCameraInput {
  name: string;
  host: string;
  username: string;
  password?: string;
  rtsp_port?: number;
  http_port?: number;
  rtsp_url?: string;
  stream_profile?: string;
  enabled?: boolean;
  device_name?: string | null;
  osd?: string | null;
}

export interface UpdateCameraInput {
  id: string;
  name?: string;
  host?: string;
  username?: string;
  password?: string;
  rtsp_port?: number;
  http_port?: number;
  rtsp_url?: string;
  stream_profile?: string;
  enabled?: boolean;
  device_name?: string | null;
  osd?: string | null;
}

export interface BatchDeviceItem {
  name: string;
  host: string;
  rtsp_port: number;
  http_port?: number;
  custom_rtsp_url?: string;
  device_name?: string | null;
  osd?: string | null;
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
  device_name?: string | null;
  osd?: string | null;
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

export type UserPermission = 'admin' | 'operator' | 'view_only' | 'unknown';

export interface DeviceCapabilities {
  can_view: boolean;
  can_change_device_name: boolean;
  can_change_osd: boolean;
  can_ptz: boolean;
  can_audio: boolean;
  can_snapshot: boolean;
  can_recording: boolean;
  user_permission: UserPermission;
  protocol_used: string;
  auth_type: string;
}

export interface QuickViewConnectInput {
  ip: string;
  mac?: string;
  rtsp_port?: number;
  http_port?: number;
  username: string;
  password?: string;
  remember_password?: boolean;
}

export interface QuickViewSetDeviceNameInput {
  ip: string;
  http_port?: number;
  username: string;
  password?: string;
  new_name: string;
}

export interface QuickViewSetOsdInput {
  ip: string;
  http_port?: number;
  channel_id?: number;
  username: string;
  password?: string;
  new_osd: string;
}

export interface QuickViewSessionInfo {
  ip: string;
  rtsp_port: number;
  http_port: number;
  brand: string;
  hardware_model: string;
  serial_number?: string;
  firmware_version?: string;
  mac_address?: string;
  device_name: string;
  osd_text: string;
  stream_url: string;
  local_mjpeg_url: string;
  capabilities: DeviceCapabilities;
  metrics: CameraConnectionTestResult;
}

export interface CachedDeviceCredentials {
  username: string;
  password: string;
}
