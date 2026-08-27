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
**Causa raiz confirmada via `strace`**: para embutir os binários grandes (`ffmpeg.exe`/`ffprobe.exe`,
~145 MB cada), o `makensis` cria um arquivo de trabalho em `/tmp` (caminho fixo no binário — a
variável de ambiente `TMPDIR` **não** é respeitada) e o cresce/mapeia em janelas de 32 MB via
`mmap(MAP_SHARED)`. Se `/tmp` for `tmpfs` (comum em desktops Linux modernos) e não tiver espaço
livre suficiente na sua própria cota — mesmo com RAM/swap do sistema sobrando — o `mmap` recebe
`SIGBUS` (`BUS_ADRERR`) ao tentar escrever numa página sem lastro. Isso **não é falta de memória do
sistema**: já foi testado e confirmado travando mesmo com RAM/swap livres de sobra; o gargalo é
especificamente a cota do próprio `/tmp`, que se enche com o acúmulo de builds/artefatos de sessões
anteriores.

**Solução**: limpe `/tmp` (ou reinicie a máquina, que remonta o `tmpfs` vazio) antes de gerar o
instalador Windows. Depois disso, o build normal funciona sem precisar de nenhum workaround:

```bash
df -h /tmp   # confira quanto está livre; se estiver quase cheio, limpe ou reinicie
pnpm tauri build --target x86_64-pc-windows-gnu
```

Evite `sudo mount -o remount,size=...` em `/tmp` como "solução rápida" — em algumas configurações
isso reseta as permissões do diretório (perde o bit `+t` / modo `1777`), quebrando a escrita nele
para todos os processos do usuário até corrigir com `sudo chmod 1777 /tmp` (ou reiniciar).
