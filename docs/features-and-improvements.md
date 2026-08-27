# OnliView - Documentação Técnica das Implementações (v0.1.9 / v0.2.0)

Este documento registra todas as funcionalidades, arquitetura, correções técnicas e melhorias implementadas no **OnliView** nas versões `v0.1.9` e `v0.2.0`.

---

## 📋 Sumário
1. [Colunas Device Name e OSD na Listagem de Dispositivos](#1-colunas-device-name-e-osd-na-listagem-de-dispositivos)
2. [Suporte Completo ISAPI para Câmeras IP e Vídeo Porteiros](#2-suporte-completo-isapi-para-câmeras-ip-e-vídeo-porteiros)
3. [Motor de Vídeo RTSP e Eliminação de Tela Preta](#3-motor-de-vídeo-rtsp-e-eliminação-de-tela-preta)
4. [Resiliência de Reprodução no Frontend](#4-resiliência-de-reprodução-no-frontend)
5. [Processo de Build e Releases Multiplataforma](#5-processo-de-build-e-releases-multiplataforma)

---

## 1. Colunas Device Name e OSD na Listagem de Dispositivos

### 🎯 Objetivo
Permitir que o técnico identifique visualmente o nome real do dispositivo (`Device Name`) e o texto gravado na imagem da câmera (`OSD`) diretamente na tabela principal de dispositivos, sabendo se o equipamento já está identificado ou não.

### 🛠️ Implementação no Banco de Dados SQLite (`src-tauri/src/database/`)
- **Migração Automática de Schema** (`schema.rs` e `repository.rs`):
  Ao iniciar, o OnliView inspeciona as colunas da tabela `cameras` via `PRAGMA table_info(cameras)` e aplica automaticamente `ALTER TABLE` caso as novas colunas não existam:
  ```sql
  ALTER TABLE cameras ADD COLUMN device_name TEXT;
  ALTER TABLE cameras ADD COLUMN osd TEXT;
  ```
- **Persistência e Leitura**:
  Os campos `device_name` e `osd` foram mapeados em todas as operações CRUD (`create_camera`, `create_cameras_batch`, `update_camera`, `get_camera_by_id`, `get_cameras`).

### 🛠️ Captura Automática no Ato do Cadastro (`src-tauri/src/camera/manager.rs`)
- Ao cadastrar uma câmera individual ou importar dispositivos em lote na Descoberta Inteligente, o backend aciona uma rotina assíncrona que:
  1. Instancia o cliente ISAPI com as credenciais fornecidas (`admin` / senha).
  2. Consulta `get_device_info()` para capturar o `deviceName`.
  3. Consulta `get_osd_title(1)` para capturar o texto do `OSD`.
  4. Salva esses metadados no banco de dados SQLite local.
- No teste de conexão de câmera (`test_connection`), esses campos também são lidos e retornados imediatamente para pré-preenchimento no formulário.

### 🎨 Interface do Usuário (Frontend React)
- **Tabela do Dashboard** (`src/pages/DashboardPage.tsx`):
  - Novas colunas visíveis: **Device Name** e **OSD**.
  - Badge visual destacando se o dispositivo possui nome/OSD configurado ou se está pendente de identificação.
  - Busca universal filtrando em tempo real por: Nome da Câmera, IP, Fabricante, Modelo, Device Name e OSD.
- **Formulários e Modais**:
  - `src/cameras/CameraModal.tsx`: Inputs dedicados para Device Name e OSD com auto-detecção via botão "Testar Conexão".
  - `src/cameras/CameraList.tsx`: Exibição dos metadados nos cards de listagem.
  - `src/cameras/DiscoveryModal.tsx` e `src/components/DiscoveryPanel.tsx`: Captura automática ao cadastrar dispositivos descobertos.

---

## 2. Suporte Completo ISAPI para Câmeras IP e Vídeo Porteiros

### 🔍 O Problema Técnico em Aparelhos de Interfonia / Vídeo Porteiro
Em câmeras IP convencionais (séries DS-2CD), o texto do OSD fica localizado dentro de `/ISAPI/System/Video/inputs/channels/1/overlays` na tag `<channelNameOverlay><name>...</name></channelNameOverlay>`.

Contudo, em **Vídeo Porteiros e Interfonia IP (como Hikvision DS-KB8112-IM)**:
1. O endpoint `/overlays` define apenas as coordenadas na tela (`positionX`, `positionY`), sem tag `<name>`.
2. O endpoint `/title` retorna `403 Forbidden / Invalid Operation`.
3. O nome do canal é definido em `/ISAPI/System/Video/inputs/channels/1` (`<VideoInputChannel><name>...</name></VideoInputChannel>`).
4. O texto **realmente desenhado pelo DSP na imagem de vídeo** é controlado pela camada **`TextOverlayList` $\rightarrow$ `TextOverlay (ID 1)`** através da tag `<displayText>`.

### 🛠️ Solução Implementada em Cascata (`src-tauri/src/camera/isapi.rs`)
O cliente ISAPI nativo em Rust foi reestruturado com suporte multi-camadas:

#### A. Leitura de OSD (`get_osd_title`)
1. **Prioridade 1**: `TextOverlay (ID 1)` em `/overlays` (lê o texto ativo impresso pelo processador de vídeo).
2. **Prioridade 2**: `channelNameOverlay` em `/overlays` (padrão de câmeras IP).
3. **Prioridade 3**: `VideoInputChannel` em `/ISAPI/System/Video/inputs/channels/{id}` (padrão interfonia).
4. **Prioridade 4**: Endpoint legado `/title`.
5. **Prioridade 5**: `StreamingChannel` (`/ISAPI/Streaming/channels/101`).

#### B. Gravação de OSD (`set_osd_title`)
Ao alterar o OSD pelo Quick Viewer ou tela de edição, o sistema grava em **todas as camadas necessárias**:
1. **Ativação do TextOverlay 1** (`patch_video_overlay_xml`):
   - Localiza `<TextOverlay>` ID 1.
   - Define `<enabled>true</enabled>`.
   - Define posição visível (`positionX: 64, positionY: 64`) caso zerada.
   - Atualiza `<displayText>NOVO_TEXTO</displayText>`.
   - Atualiza `<channelNameOverlay>` com `<enabled>true</enabled>`.
   - Envia `PUT /ISAPI/System/Video/inputs/channels/{id}/overlays`.
2. **Atualização de Entrada de Vídeo**:
   - Envia `PUT /ISAPI/System/Video/inputs/channels/{id}` com `<name>NOVO_TEXTO</name>`.
3. **Atualização Legada**:
   - Envia `PUT /ISAPI/System/Video/inputs/channels/{id}/title`.
4. **Sincronização no Banco Local**:
   - Atualiza imediatamente o registro da câmera no SQLite local se já cadastrada.

---

## 3. Motor de Vídeo RTSP e Eliminação de Tela Preta

### 🔍 Diagnóstico das Falhas no Windows
A análise de telemetria e logs (`docs/LOGS-OLIFIN-CFTV.txt`) revelou as causas exatas das telas pretas em ambiente Windows:
- **Smart Codec H.265 / H.265+**: Em cenas com pouco movimento, o encoder da câmera reduz a frequência de **I-Frames (Keyframes)** para cada 2 a 4 segundos. O FFmpeg necessita do I-Frame para inicializar a tabela de referências (RPS) do HEVC. Com timeouts muito curtos (3s), o processo abortava antes do primeiro I-Frame chegar.
- **Negociação de Transporte RTSP (TCP vs UDP)**: Algumas câmeras ou redes com firewall/NAT travam o transporte entrelaçado TCP, funcionando exclusivamente sobre UDP (ou vice-versa).
- **Atraso de Conexão no Servidor Local MJPEG**: O WebView2 (Chromium no Windows) abria a conexão HTTP antes do primeiro frame ter sido gerado pelo FFmpeg, disparando `onError` na tag `<img>`.

### 🛠️ Melhorias Aplicadas no Backend Rust

#### A. Otimização dos Parâmetros do FFmpeg (`src-tauri/src/video/engine.rs`)
```rust
"-hide_banner",
"-loglevel", "warning",
"-rtsp_transport", transport,                 // TCP na 1ª tentativa, UDP no fallback
"-stimeout", "6000000",                       // 6 segundos de socket timeout para acomodar GOPs de H.265
"-fflags", "+nobuffer+discardcorrupt+genpts", // Corrige timestamps PTS e descarta pacotes corrompidos
"-flags", "low_delay",                        // Reduz latência de buffer
"-max_delay", "500000",                       // Máximo de 0.5s de atraso no demuxer
"-analyzeduration", "2000000",                // 2s de análise para capturar VPS/SPS/PPS em H.265+
"-probesize", "2000000",                      // 2 MB de buffer de inspeção
"-threads", "2",                              // Decodificação multi-thread para fluxos pesados
"-i", &rtsp_url,
"-an",
"-c:v", "mjpeg",
"-q:v", "4",
"-r", "25",
"-f", "image2pipe",
"-vcodec", "mjpeg",
"pipe:1"
```

#### B. Fallback Dinâmico de Transporte (TCP $\leftrightarrow$ UDP)
Caso a conexão RTSP caia ou falhe na inicialização, o motor alterna automaticamente entre `TCP` e `UDP` a cada tentativa de reconexão.

#### C. Entrega Imediata de Frame Inicial em Cache (`src-tauri/src/video/stream_server.rs`)
No manipulador `mjpeg_stream_handler`:
```rust
let initial_frame = manager.get_latest_frame(&camera_id).await;
let initial_stream = tokio_stream::iter(initial_frame.into_iter().map(|frame| {
    // Monta o cabeçalho multipart e o payload do frame JPEG imediatamente
    ...
}));
let combined_stream = initial_stream.chain(broadcast_stream);
```
Assim que a tag `<img>` conecta, ela recebe o último frame válido em cache de forma instantânea, eliminando a espera e o risco de tela preta inicial.

#### D. Descoberta e Priorização de Substream no Quick Viewer (`src-tauri/src/camera/manager.rs`)
O Quick Viewer passa a buscar prioritariamente o canal secundário (`/Streaming/Channels/102`), que decodifica em menos de 100ms em H.264 leve (~512 kbps), com fallback transparente para o canal 101.

---

## 4. Resiliência de Reprodução no Frontend

### 🛠️ Implementação nos Componentes React (`QuickViewerModal.tsx` & `VideoCell.tsx`)
- **Mecanismo de Auto-Retry com Debounce**:
  Em vez de bloquear a célula imediatamente ao primeiro erro de rede, o componente dispara até 4 tentativas automáticas de recarregamento com debounce de 1.2 segundos:
  ```typescript
  const handleImageError = () => {
    if (retryCountRef.current < 4) {
      retryCountRef.current += 1;
      if (retryTimeoutRef.current) clearTimeout(retryTimeoutRef.current);
      retryTimeoutRef.current = setTimeout(() => {
        setReloadKey(Date.now());
      }, 1200);
    } else {
      setImageError(true);
    }
  };
  ```
- **Reset no Reconnect Manual**:
  O contador é zerado ao reconectar manualmente ou ao abrir nova sessão.

---

## 5. Processo de Build e Releases Multiplataforma

### 📦 Artefatos Gerados
| Plataforma | Formato | Caminho / Destino | Descrição |
| :--- | :--- | :--- | :--- |
| **Windows** | `.exe` (NSIS) | `release-windows/OnliView_0.2.0_x64-setup.exe` | Instalador oficial com binários do FFmpeg embutidos |
| **Windows** | `.zip` (Portable) | `release-windows/OnliView_Windows_Portable_v0.2.0.zip` | Versão portátil compactada sem necessidade de instalação |
| **Linux** | `.deb` | `release-packages/OnliView_0.2.0_amd64.deb` | Pacote nativo para Debian / Ubuntu / Linux Mint |
| **Linux** | `.AppImage` | `release-packages/OnliView_0.2.0_amd64.AppImage` | Executável universal portátil para distribuições Linux |
| **Linux** | `.rpm` | `release-packages/OnliView-0.2.0-1.x86_64.rpm` | Pacote para Fedora / Red Hat / CentOS |

### 🚀 Publicação no GitHub Releases
A publicação é automatizada utilizando o GitHub CLI (`gh`), anexando todos os executáveis e documentando o changelog detalhado em cada versão:
- **v0.1.9**: [https://github.com/onlitec/OnliCFTV/releases/tag/v0.1.9](https://github.com/onlitec/OnliCFTV/releases/tag/v0.1.9)
- **v0.2.0**: [https://github.com/onlitec/OnliCFTV/releases/tag/v0.2.0](https://github.com/onlitec/OnliCFTV/releases/tag/v0.2.0)
