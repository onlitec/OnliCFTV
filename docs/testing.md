# Plano e Bateria de Testes - OnliView

## 1. Matriz de Cenários de Teste

| Cenário de Teste | Comportamento Esperado | Status |
|---|---|---|
| **Câmera Online (Credenciais Válidas)** | Conexão estabelecida, codec detectado, stream exibido no LiveView, FPS contabilizado | Validado |
| **Câmera Offline (Host Inacessível)** | Timeout controlado de 5s, status "Câmera offline" exibido com botão de reconexão | Validado |
| **IP Inválido / Host Desconhecido** | Erro de resolução reportado claramente sem travamento da aplicação | Validado |
| **Porta RTSP Fechada / Recusada** | Mensagem "Conexão recusada (Verifique IP e porta 554)" retornada | Validado |
| **Usuário ou Senha Inválidos (Digest/Basic)** | Retorno "Falha na autenticação RTSP (Usuário ou senha incorretos)" | Validado |
| **RTSP URI Inexistente** | Diagnóstico reporta erro de stream e não inicia loop infinito | Validado |
| **Queda de Rede / Desconexão** | Estado transita para Offline e reconexão automática é disparada | Validado |
| **Reconexão Automática** | Quando o sinal retorna, o vídeo volta a fluir automaticamente | Validado |
| **Sanitização de Credenciais em Logs** | Nenhuma senha é impressa em tela ou logs (`rtsp://user:***@host`) | Validado |
| **Criptografia SQLite** | Senhas são salvas cifradas em AES-256-GCM no arquivo `.db` | Validado |

## 2. Teste com Equipamento de Laboratório Hikvision
- **Host de Laboratório**: `172.20.120.67`
- **Porta RTSP**: `554`
- **Usuário**: `admin`
- **Autenticação**: Digest RTSP
