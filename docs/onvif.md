# Especificação da Camada ONVIF - OnliView

## 1. Visão Geral
A arquitetura do OnliView inclui uma camada desacoplada (`src-tauri/src/onvif/`) preparada para suportar o protocolo ONVIF Profile S/T/G nas próximas etapas do projeto.

## 2. Roadmap ONVIF
- **Fase 1 (MVP Atual)**: Esqueleto estrutural com modelos de dispositivos, perfis e resolução de URIs RTSP.
- **Fase 2**: Descoberta automática via WS-Discovery (UDP broadcast em `239.255.255.250:3702`).
- **Fase 3**: Consulta de Device Information (Fabricante, Modelo, Firmware, Serial).
- **Fase 4**: Consulta de Profiles de Mídia e URLs RTSP automáticas.
- **Fase 5**: Controle PTZ (Pan, Tilt, Zoom, Presets) via SOAP/XML.
