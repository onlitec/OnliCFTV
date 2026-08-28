//! Verificação de gravações em NVRs.
//!
//! Responde, para cada canal de cada NVR cadastrado, se há vídeo gravado no
//! período — e cruza isso com as câmeras cadastradas para revelar dois
//! descompassos: canal no NVR sem câmera cadastrada, e câmera cadastrada que não
//! está em NVR nenhum.
//!
//! Nada aqui é persistido: o NVR é a fonte da verdade e cada consulta é ao vivo.

use std::net::Ipv4Addr;

use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::camera::isapi::RecordingSegment;
use crate::camera::model::Camera;

/// Resultados por página na busca do NVR. Mantém cada resposta pequena, longe do
/// teto de 1 MiB do leitor HTTP.
pub const SEARCH_PAGE_SIZE: u16 = 40;

/// Teto de páginas por canal. Uma câmera com gravação por movimento pode ter
/// milhares de segmentos; sem esse limite, um único canal atrasaria a varredura
/// do site inteiro. Ao estourar, o resultado vem marcado como `truncated`.
pub const MAX_SEARCH_PAGES: u32 = 8;

/// Requisições simultâneas dentro de um mesmo NVR. Servidores web embarcados
/// suportam poucas sessões HTTP concorrentes, compartilhadas com a própria
/// interface do aparelho — paralelismo total pode deixar o técnico sem acesso ao
/// NVR no meio da verificação.
pub const MAX_CONCURRENT_PER_NVR: usize = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelRecordingStatus {
    pub channel_id: u32,
    pub channel_name: String,
    /// `None` em canal analógico/local (DVR/híbrido).
    pub ip_address: Option<String>,
    pub online: Option<bool>,
    pub matched_camera_id: Option<String>,
    pub matched_camera_name: Option<String>,
    /// `None` = não foi possível determinar (erro, sem permissão, não suportado).
    /// Distinguir isso de `Some(false)` importa: "não gravou" e "não consegui
    /// perguntar" levam o técnico a ações diferentes.
    pub is_recording: Option<bool>,
    pub segments: Vec<RecordingSegment>,
    pub coverage_ratio: f32,
    pub truncated: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NvrRecordingReport {
    pub nvr_id: String,
    pub nvr_name: String,
    pub nvr_host: String,
    pub reachable: bool,
    pub auth_ok: bool,
    pub error: Option<String>,
    /// Canais casados com uma câmera cadastrada.
    pub channels: Vec<ChannelRecordingStatus>,
    /// Canais presentes no NVR sem câmera correspondente no cadastro.
    pub unregistered_channels: Vec<ChannelRecordingStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingCheckResult {
    pub period_start: String,
    pub period_end: String,
    pub nvr_reports: Vec<NvrRecordingReport>,
    /// Câmeras cadastradas que não apareceram em nenhum NVR consultado.
    pub orphan_cameras: Vec<Camera>,
}

/// Converte um horário devolvido pelo ISAPI.
///
/// Aceita RFC3339 (`...Z` ou com offset) e a variante sem fuso que algumas
/// firmwares emitem, tratada como UTC.
pub fn parse_isapi_time(s: &str) -> Option<DateTime<Utc>> {
    let t = s.trim();
    if t.is_empty() {
        return None;
    }

    if let Ok(dt) = DateTime::parse_from_rfc3339(t) {
        return Some(dt.with_timezone(&Utc));
    }

    for fmt in ["%Y-%m-%dT%H:%M:%S", "%Y-%m-%d %H:%M:%S"] {
        if let Ok(naive) = NaiveDateTime::parse_from_str(t, fmt) {
            return Some(DateTime::from_naive_utc_and_offset(naive, Utc));
        }
    }

    None
}

/// Formata um instante no formato que o ISAPI espera na busca.
pub fn format_isapi_time(dt: DateTime<Utc>) -> String {
    dt.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// Fração do período (0.0–1.0) coberta por gravação.
///
/// Segmentos são recortados ao período e sobreposições são mescladas, senão dois
/// segmentos sobrepostos somariam mais tempo do que o período tem e a barra de
/// cobertura passaria de 100%.
pub fn coverage_ratio(
    segments: &[RecordingSegment],
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
) -> f32 {
    let total = (period_end - period_start).num_seconds();
    if total <= 0 {
        return 0.0;
    }

    let mut spans: Vec<(i64, i64)> = segments
        .iter()
        .filter_map(|s| {
            let start = parse_isapi_time(&s.start)?;
            let end = parse_isapi_time(&s.end)?;
            // Recorta ao período consultado.
            let a = start.max(period_start);
            let b = end.min(period_end);
            if b <= a {
                return None;
            }
            Some((
                (a - period_start).num_seconds(),
                (b - period_start).num_seconds(),
            ))
        })
        .collect();

    if spans.is_empty() {
        return 0.0;
    }

    spans.sort_by_key(|(a, _)| *a);

    let mut covered = 0i64;
    let (mut cur_start, mut cur_end) = spans[0];
    for &(a, b) in &spans[1..] {
        if a > cur_end {
            covered += cur_end - cur_start;
            cur_start = a;
            cur_end = b;
        } else if b > cur_end {
            cur_end = b;
        }
    }
    covered += cur_end - cur_start;

    (covered as f32 / total as f32).clamp(0.0, 1.0)
}

/// Casa o IP de um canal do NVR com uma câmera cadastrada.
///
/// Compara `Ipv4Addr` já convertido, e não a string crua, para que diferença de
/// formatação vinda da firmware não gere um falso "câmera não cadastrada".
/// Cai para comparação textual quando algum dos lados não é IPv4 (hostname).
pub fn match_camera_by_ip<'a>(channel_ip: &str, cameras: &'a [Camera]) -> Option<&'a Camera> {
    let target = channel_ip.trim();
    if target.is_empty() {
        return None;
    }

    if let Ok(parsed) = target.parse::<Ipv4Addr>() {
        if let Some(cam) = cameras
            .iter()
            .find(|c| c.host.trim().parse::<Ipv4Addr>() == Ok(parsed))
        {
            return Some(cam);
        }
    }

    cameras
        .iter()
        .find(|c| c.host.trim().eq_ignore_ascii_case(target))
}

/// Número do track de gravação do fluxo principal de um canal.
/// Convenção Hikvision: canal 1 → 101, canal 2 → 201.
pub fn track_id_for_channel(channel_id: u32) -> u32 {
    channel_id * 100 + 1
}
