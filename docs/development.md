# Guia de Desenvolvimento - OnliView

## 1. Pré-requisitos
- **Ubuntu Linux 24.04 / 26.04** ou **Windows 10/11**
- **Rust** 1.80+ (`rustup toolchain stable`)
- **Node.js** 20+ LTS e **pnpm** 9+
- **FFmpeg & ffprobe** 6.0+ instalados no PATH
- **SQLite3** e `libsqlite3-dev`
- **Bibliotecas do Tauri (Linux)**: `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, `patchelf`.

## 2. Estrutura do Repositório
```
onliview/
├── src/                  # Frontend React + TypeScript
│   ├── components/       # Componentes de interface (Sidebar, Header, etc.)
│   ├── pages/            # Páginas da aplicação (Dashboard, Gravações, Eventos)
│   ├── cameras/          # Gerenciamento e cadastro de câmeras (Lista, Modal)
│   ├── video/            # Mosaico de vídeo (LiveView, VideoCell)
│   ├── diagnostics/      # Telemetria técnica e visualizador de logs
│   ├── settings/         # Configurações do sistema
│   ├── services/         # Camada de comunicação IPC com o Tauri
│   └── types/            # Definições TypeScript
├── src-tauri/             # Backend Nativo Rust
│   ├── src/
│   │   ├── camera/       # Modelos, gerenciador e criptografia AES-GCM
│   │   ├── rtsp/         # Utilitários RTSP e sonda de conexão (ffprobe)
│   │   ├── onvif/        # Esqueleto de descoberta ONVIF
│   │   ├── video/        # VideoEngine, orquestrador de streams e servidor Axum
│   │   ├── database/     # SQLite schema, migrações e repositório
│   │   ├── configuration/# Configurações locais da aplicação
│   │   └── logging/      # Logger estruturado e sanitizador de senhas
│   ├── tauri.conf.json   # Configurações do Tauri v2
│   └── Cargo.toml        # Dependências Rust
├── database/             # Diretório reservado para banco de dados local
└── docs/                 # Documentação técnica e arquitetural
```

## 3. Comandos Úteis

### Executar em modo desenvolvimento:
```bash
pnpm tauri dev
```

### Compilar o Frontend:
```bash
pnpm build
```

### Verificar compilação do Backend Rust:
```bash
cd src-tauri && cargo check
```

### Gerar executável de produção (.deb / AppImage):
```bash
pnpm tauri build
```

### Build Windows (.exe) — atenção ao FFmpeg embutido
`src-tauri/resources/ffmpeg.exe` e `src-tauri/resources/ffprobe.exe` são propositalmente ignorados pelo git
(`*.exe` no `.gitignore`, por serem binários grandes — ver `video/bin_locator.rs`). Um checkout limpo
**não** os contém. Antes de rodar `pnpm tauri build --target x86_64-pc-windows-gnu`, copie manualmente
`ffmpeg.exe` e `ffprobe.exe` (build estático Windows x64) para `src-tauri/resources/`. Sem eles, o instalador
gerado não terá o motor de vídeo embutido e, na máquina do usuário final (sem ffmpeg no PATH), o Live View
não exibirá imagem — apenas o log de Diagnóstico mostrará "Binário FFmpeg não encontrado no bundle/local
esperado".

### Build Windows trava com "Erro no barramento" / SIGBUS ao empacotar (`makensis`)
Se o projeto estiver em um disco montado via FUSE (ex.: `fuseblk`/NTFS via `ntfs-3g`, `/mnt/...`), o
`makensis` pode travar com SIGBUS especificamente ao embutir os binários grandes (`ffmpeg.exe`/`ffprobe.exe`,
~145 MB cada) — mmap de arquivos grandes sobre FUSE não é confiável, independente de RAM livre ou do
algoritmo de compressão (`tauri.conf.json` usa `"compression": "zlib"` para reduzir o consumo de memória,
mas isso sozinho **não** resolve — testado e confirmado ainda travando em FUSE mesmo com zlib e memória
livre). Se isso acontecer, contorne rodando o empacotamento a partir de um disco local (ext4/etc.):

```bash
# 1. Gere o build normal primeiro (compila o binário e o script .nsi)
pnpm tauri build --target x86_64-pc-windows-gnu   # provavelmente falha na etapa do makensis, tudo bem

# 2. Copie o script gerado + os binários grandes para um disco local
STAGE=~/.onliview-nsis-local
mkdir -p "$STAGE/bin"
cp src-tauri/target/x86_64-pc-windows-gnu/release/nsis/x64/*.nsi "$STAGE/"
cp src-tauri/target/x86_64-pc-windows-gnu/release/nsis/x64/*.nsh "$STAGE/"
cp src-tauri/target/x86_64-pc-windows-gnu/release/onliview.exe "$STAGE/bin/"
cp src-tauri/target/x86_64-pc-windows-gnu/release/WebView2Loader.dll "$STAGE/bin/"
cp src-tauri/resources/ffmpeg.exe src-tauri/resources/ffprobe.exe "$STAGE/bin/"

# 3. Ajuste os caminhos absolutos dentro do installer.nsi copiado para apontar para $STAGE/bin/...
#    (edite manualmente as linhas MAINBINARYSRCPATH, OUTFILE e os dois `File /a "/oname=resources\..."`)

# 4. Rode o makensis local (fora do FUSE) e copie o instalador resultante de volta para release-windows/
cd "$STAGE" && makensis -INPUTCHARSET UTF8 installer.nsi
```
