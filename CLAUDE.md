# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

**OnliView** (package/binary name `onliview`) is Onlitec's desktop VMS (Video Management System) for
IP CFTV commissioning and live monitoring — a Tauri v2 app: React/TypeScript frontend + native Rust backend.
Primary focus is Hikvision IP cameras/NVRs (SADP + ISAPI + RTSP), with ONVIF and generic RTSP discovery support.
Targets Linux (.deb/.AppImage/.rpm) and Windows (.exe NSIS installer / portable ZIP).

## Commands

Package manager is **pnpm** (see `pnpm-workspace.yaml`, `.npmrc`). There is no lint or single-test-by-name
script configured; tests are run via `cargo test` (see below).

```bash
# Install deps
pnpm install

# Run full app in dev mode (frontend + Tauri/Rust backend)
pnpm tauri dev

# Frontend only (Vite dev server on port 1420, expects Tauri IPC — will not fully work standalone)
pnpm dev

# Type-check + build frontend (tsc && vite build)
pnpm build

# Rust: fast compile check (run from src-tauri/)
cd src-tauri && cargo check

# Rust: run backend integration tests (single test file: tests/integration_tests.rs)
cd src-tauri && cargo test
cd src-tauri && cargo test test_crypto_roundtrip   # run a single test by name

# Production build (native platform: .deb/.AppImage/.rpm on Linux)
pnpm tauri build

# Cross-compile Windows installer from Linux (requires mingw-w64 toolchain, see src-tauri/.cargo/config.toml)
pnpm tauri build --target x86_64-pc-windows-gnu
```

Native prerequisites (see `docs/development.md`): Rust 1.80+, Node 20+/pnpm 9+, FFmpeg & ffprobe on PATH,
SQLite3, and on Linux the Tauri WebKitGTK/GTK3 dev libraries. On Windows, `ffmpeg.exe`/`ffprobe.exe` are
bundled from `src-tauri/resources/` (see `video/bin_locator.rs`, which searches next to the exe, in
`resources/`, `bin/`, then falls back to PATH).

## Architecture

IPC boundary: the React frontend never talks to hardware directly — every camera/discovery/video/log
operation goes through a Tauri `#[tauri::command]` in `src-tauri/src/lib.rs`, invoked from the frontend via
the single wrapper `src/services/api.ts` (thin `invoke('command_name', {...})` calls typed against
`src/types/index.ts`). When adding a backend capability: add the Rust command in `lib.rs`, register it in
the `tauri::generate_handler![...]` list, then add a matching method in `api.ts` + types in `types/index.ts`.

All shared backend state lives in one `AppState` (`lib.rs`): `config: AppConfig`, `camera_manager:
CameraManager`, `video_engine: VideoEngineManager`, `log_store: LogStore`. Commands are thin — they just
delegate into these managers.

### Backend module map (`src-tauri/src/`)

- `camera/` — `manager.rs` (CameraManager: CRUD, connection tests, Quick View sessions — orchestrates DB +
  video engine + ISAPI), `model.rs` (Camera/CreateCameraInput/etc. structs mirrored on the TS side),
  `isapi.rs` (Hikvision ISAPI client — Digest auth, device name & OSD read/write), `crypto.rs`
  (AES-256-GCM password encryption; key derived from hostname+user+machine-id, so encrypted DB values are
  **not portable across machines**).
- `discovery/` — multi-protocol device discovery engine. `engine.rs` orchestrates parallel providers under
  `providers/` (`sadp.rs` Hikvision SADP UDP 37020, `onvif.rs` WS-Discovery UDP 3702, `arp.rs` OUI lookup,
  `tcp.rs`/`icmp.rs`/`http.rs` port/banner probing) with a bounded-concurrency (48 workers) TCP sweep.
  `classifier.rs` scores evidence across providers to separate real cameras from false positives (e.g.
  Linux servers, switches) — see `test_ubuntu_server_classification_not_camera` for the kind of case it
  guards against. `deduplicator.rs` merges duplicate sightings of the same device across protocols.
- `video/` — `engine.rs` (`VideoEngineManager`/`CameraStreamStatus`: per-camera stream session lifecycle,
  auto-reconnect with exponential backoff, frame distribution), `stream_server.rs` (local Axum HTTP server
  on port **18554** serving MJPEG multipart streams consumed by the frontend `LiveView`/`VideoCell`),
  `bin_locator.rs` (locates ffmpeg/ffprobe across dev/bundled/system locations).
- `rtsp/` — `client.rs` (builds authenticated RTSP URLs), `probe.rs` (ffprobe-based telemetry: codec,
  resolution, FPS, with a 5s timeout).
- `onvif/` — ONVIF discovery skeleton, separate from the `discovery/providers/onvif.rs` WS-Discovery probe.
- `database/` — `schema.rs` (SQLite DDL, WAL mode), `repository.rs`, `mod.rs` (`Database`, cloneable handle
  shared across managers).
- `logging/logger.rs` — `LogStore` (in-memory ring buffer of structured `LogEntry`s surfaced to the
  Diagnostics UI) and `sanitize_credentials` (redacts passwords from RTSP URLs, e.g.
  `rtsp://user:***@host`, before anything is logged) — **always route new log lines through the
  sanitizer**, never log a raw camera/device URL or credential.
- `configuration/config.rs` — `AppConfig` (DB path, video server port, etc.), currently hardcoded defaults
  (no user-facing settings persistence beyond what `SettingsPage.tsx` displays read-only).

### Frontend module map (`src/`)

- `services/api.ts` — the only file that calls `invoke()`; all IPC goes through here.
- `types/index.ts` — TypeScript mirrors of the Rust command payload/return types; keep in sync manually
  when Rust structs change.
- `cameras/` — camera registry UI: `CameraList.tsx` (high-density table, batch operations), `CameraModal.tsx`
  (create/edit form), `DiscoveryModal.tsx` (runs discovery, lets user commission found devices).
  `components/DeviceThumbnailCell.tsx` and `DiscoveryPanel.tsx` implement on-demand preview thumbnails
  (streams start only when requested, with a configurable 1–6 concurrent stream cap) and in-cell password
  prompts for devices needing activation.
  `components/QuickViewerModal.tsx` is the field-commissioning quick viewer: low-delay RTSP preview served
  via the local MJPEG server, plus direct ISAPI Device Name/OSD editing without opening a browser.
  `video/LiveView.tsx` + `VideoCell.tsx` render the monitoring mosaic (1x1/2x2/3x3 grids) against
  `get_stream_status`/`get_all_stream_statuses`.
- `diagnostics/DiagnosticsPage.tsx` — reads `get_logs`/`clear_logs`, filterable by level.
- `context/ThemeContext.tsx` — light/dark theme, persisted to `localStorage`.
- `layouts/MainLayout.tsx`, `components/Sidebar.tsx`/`Header.tsx` — app shell/navigation.
- Path alias `@/*` maps to `src/*` (configured in both `tsconfig.json` and `vite.config.ts`).

### Security-sensitive invariants

- Camera passwords are always encrypted (`camera/crypto.rs`, AES-256-GCM) before hitting SQLite — never
  store or return plaintext passwords from a new code path.
- Any new logging call touching a device URL/credential must go through `logging::logger::sanitize_credentials`.
- The discovery classifier exists specifically to avoid false-positive "camera" detections on generic
  network hosts — when touching `discovery/classifier.rs`, preserve/extend the existing test coverage in
  `src-tauri/tests/integration_tests.rs` rather than loosening scoring heuristics.

## Repo conventions

- `dist/`, `src-tauri/target/`, `release-windows/`, `release-packages/`, and built installers (`*.exe`,
  `*.msi`, `*.dll`) are gitignored / kept out of the tree — built binaries live in GitHub Releases, not git
  (see recent commit "desvincular binários pesados da árvore git").
- Documentation in Portuguese lives under `docs/` (`architecture.md`, `development.md`, `testing.md`,
  `onvif.md`, `rtsp.md`) — check there first for deeper protocol-level detail (ONVIF, RTSP) before
  re-deriving it from source.
- README and docs are written in Portuguese (pt-BR); match that language when editing user-facing docs.
