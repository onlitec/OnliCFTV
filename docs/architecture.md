# OnliView - Arquitetura do Sistema

## 1. Visão Geral
O **OnliView** é um aplicativo desktop de Video Management System (VMS) e monitoramento de CFTV IP profissional desenvolvido pela **Onlitec**. O objetivo é prover visualização em tempo real de câmeras IP, NVRs e videoporteiros (com foco inicial em equipamentos Hikvision) sem depender de navegadores comerciais (Chrome/Firefox), plugins NPAPI legados ou Web Components proprietários.

## 2. Diagrama Arquitetural
```mermaid
graph TD
    subgraph Frontend [Frontend Desktop UI]
        React[React 18 + TypeScript]
        Tailwind[Tailwind CSS + shadcn/ui]
        LiveView[Mosaico Live View 1x1 / 2x2 / 3x3]
        DiagUI[Diagnóstico & Telemetria]
    end

    subgraph TauriBridge [Camada de Comunicação Tauri v2]
        IPC[Tauri IPC Commands]
        HTTPStream[Local MJPEG Stream Client]
    end

    subgraph NativeBackend [Backend Nativo Rust]
        CamMgr[Camera Manager]
        SQLite[(SQLite Database WAL Mode)]
        Crypto[AES-256-GCM Credential Encryption]
        Logger[Structured Logger & Sanitizer]
        VideoEngine[Video Engine Abstraction]
        StreamServer[Axum HTTP Multipart Stream Server]
        OnvifSkeleton[ONVIF Discovery Layer]
    end

    subgraph ExternalEngine [Processamento de Vídeo & Dispositivos]
        FFmpeg[FFmpeg 8 Motor Nativo]
        Hikvision[Câmeras IP / NVRs Hikvision RTSP:554]
    end

    React -->|Invocação IPC| IPC
    IPC --> CamMgr
    CamMgr --> SQLite
    CamMgr --> Crypto
    CamMgr --> VideoEngine
    VideoEngine --> FFmpeg
    FFmpeg -->|RTSP TCP Basic/Digest| Hikvision
    FFmpeg -->|Frames Pipe MJPEG| VideoEngine
    VideoEngine --> StreamServer
    LiveView -->|HTTP Multipart Stream| StreamServer
    Logger --> DiagUI
```

## 3. Componentes Principais

### 3.1. Frontend (React + TypeScript + Vite)
- **Dashboard**: Painel executivo com métricas de câmeras online/offline e status do motor.
- **Câmeras**: Cadastro com formulário completo, seleção de perfil RTSP (Principal 101 / Secundário 102 / Personalizado) e botão de teste de conexão com detecção de codec e resolução.
- **Visualização (Live View)**: Grades dinâmicas (1x1, 2x2, 3x3), telemetria em tempo real (FPS, bitrate), badge de status (Online, Conectando, Offline, Erro) e reconexão automática/manual.
- **Diagnóstico**: Tabela de telemetria e console de logs estruturados e sanitizados com busca e filtro por nível (INFO, WARN, ERROR, DEBUG).
- **Configurações**: Exibição de caminhos do banco SQLite, status de criptografia e parâmetros do servidor local.

### 3.2. Backend Nativo (Rust + Tauri v2)
- **Database (`database/repository.rs`)**: Banco SQLite com `rusqlite`, modo WAL ativado para alta concorrência e integridade transacional.
- **Segurança de Credenciais (`camera/crypto.rs`)**: Criptografia AES-256-GCM com chave derivada exclusivamente da máquina local (hostname + user + machine-id), impedindo o vazamento de senhas em texto puro.
- **Log Sanitizer (`logging/logger.rs`)**: Oculta automaticamente senhas e tokens de URLs RTSP (`rtsp://user:***@host`) nos logs do sistema.
- **Motor de Vídeo Abstrato (`video/engine.rs`)**: Orquestra sessões de stream por câmera, gerencia ciclo de vida, reconexão automática com backoff exponencial e distribuição de frames.
- **Servidor de Stream Local (`video/stream_server.rs`)**: Servidor HTTP local leve baseado em `axum` e `tokio-stream` que distribui o stream MJPEG para a interface sem intermediários na porta `18554`.
- **RTSP Probe (`rtsp/probe.rs`)**: Inspeção técnica prévia via `ffprobe` com transporte TCP e timeout de 5 segundos.

## 4. Multiplataforma (Linux & Windows)
O projeto foi estruturado para compilação nativa:
- **Linux**: Pacote `.deb` e `AppImage` utilizando WebKitGTK 4.1 e GTK3.
- **Windows**: Executável `.exe` e instalador MSI/NSIS utilizando WebView2.
