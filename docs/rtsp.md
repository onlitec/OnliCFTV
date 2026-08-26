# Especificação do Motor RTSP - OnliView

## 1. Padrões Hikvision Suportados
O OnliView suporta nativamente a estrutura de URIs RTSP da Hikvision e de outros fabricantes padrão:

- **Main Stream (Alta Resolução)**:
  `rtsp://[username:password@]IP:554/Streaming/Channels/101`
- **Sub Stream (Baixa Resolução / Mosaicos)**:
  `rtsp://[username:password@]IP:554/Streaming/Channels/102`
- **Canais adicionais / NVRs multi-canais**:
  `rtsp://[username:password@]IP:554/Streaming/Channels/{channel_id}01` (Ex: canal 2 = `201`)
- **URIs Personalizadas**:
  Permite qualquer caminho customizado (ex: `rtsp://IP:554/live/ch0`, `rtsp://IP:554/onvif1`).

## 2. Autenticação RTSP
- **Basic Authentication**: Suportada.
- **Digest Authentication**: Suportada nativamente pelo backend via FFmpeg TCP handshake.

## 3. Flags de Baixa Latência no FFmpeg
Para garantir latência inferior a 150ms na reprodução ao vivo de CFTV, o processo FFmpeg é inicializado com as seguintes opções:
```bash
ffmpeg -hide_banner -loglevel error \
  -rtsp_transport tcp \
  -timeout 5000000 \
  -fflags nobuffer+discardcorrupt \
  -flags low_delay \
  -i "rtsp://user:pass@host:554/Streaming/Channels/101" \
  -an \
  -c:v mjpeg \
  -q:v 5 \
  -r 20 \
  -f image2pipe \
  -vcodec mjpeg pipe:1
```

## 4. Estratégia de Reconexão Automática
Quando o fluxo de rede é interrompido:
1. O leitor assíncrono detecta EOF ou timeout no socket TCP.
2. O estado da câmera é atualizado para `Offline`.
3. O log sanitizado registra a desconexão.
4. Uma tarefa em segundo plano tenta restabelecer o handshake a cada 5 segundos com suporte a cancelamento sob demanda.
