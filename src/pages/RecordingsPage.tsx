import React, { useMemo, useState } from 'react';
import {
  Film,
  Search,
  RefreshCw,
  Loader2,
  AlertTriangle,
  Download,
  Copy,
  ServerCrash,
  KeyRound,
  HardDriveDownload,
} from 'lucide-react';
import type {
  Camera,
  ChannelRecordingStatus,
  RecordingCheckResult,
  RecordingSegment,
} from '@/types';
import { api } from '@/services/api';

interface RecordingsPageProps {
  cameras: Camera[];
}

type PeriodPreset = 6 | 24 | 72;
type StatusFilter = 'all' | 'recording' | 'not_recording' | 'unregistered' | 'unknown';

/** Uma linha da tabela: um canal achatado junto do NVR de onde veio. */
interface Row {
  nvrName: string;
  nvrHost: string;
  registered: boolean;
  ch: ChannelRecordingStatus;
}

const pct = (v: number) => `${Math.round(v * 100)}%`;

/** Horário local curto; devolve o texto cru se não for uma data reconhecível. */
const fmtTime = (iso?: string | null): string => {
  if (!iso) return '—';
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleString();
};

/**
 * Barra de cobertura: as faixas gravadas posicionadas em porcentagem do período,
 * de modo que os buracos apareçam como espaço vazio.
 */
const CoverageBar: React.FC<{
  segments: RecordingSegment[];
  periodStart: string;
  periodEnd: string;
  ratio: number;
  truncated: boolean;
}> = ({ segments, periodStart, periodEnd, ratio, truncated }) => {
  const t0 = new Date(periodStart).getTime();
  const t1 = new Date(periodEnd).getTime();
  const span = t1 - t0;

  const blocks =
    span > 0
      ? segments
          .map((s) => {
            const a = new Date(s.start).getTime();
            const b = new Date(s.end).getTime();
            if (Number.isNaN(a) || Number.isNaN(b)) return null;
            const left = Math.max(0, ((a - t0) / span) * 100);
            const right = Math.min(100, ((b - t0) / span) * 100);
            if (right <= left) return null;
            // Piso de 0.4% para que um segmento curto ainda seja visível.
            return { left, width: Math.max(0.4, right - left) };
          })
          .filter((b): b is { left: number; width: number } => b !== null)
      : [];

  return (
    <div className="flex items-center gap-2 min-w-[150px]">
      <div
        className="relative h-2.5 flex-1 rounded-full bg-slate-200 dark:bg-slate-800 overflow-hidden"
        title={`${segments.length} trecho(s) gravado(s) — ${pct(ratio)} do período`}
      >
        {blocks.map((b, i) => (
          <div
            key={i}
            className="absolute inset-y-0 bg-emerald-500"
            style={{ left: `${b.left}%`, width: `${b.width}%` }}
          />
        ))}
      </div>
      <span className="text-[10px] font-mono text-slate-500 dark:text-slate-400 w-9 text-right">
        {pct(ratio)}
      </span>
      {truncated && (
        <span
          className="text-[10px] text-amber-600 dark:text-amber-400"
          title="Muitos trechos: exibindo apenas uma amostra do período"
        >
          ~
        </span>
      )}
    </div>
  );
};

const RecordingBadge: React.FC<{ ch: ChannelRecordingStatus }> = ({ ch }) => {
  if (ch.is_recording === true) {
    return (
      <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-semibold bg-emerald-50 dark:bg-emerald-500/15 text-emerald-700 dark:text-emerald-300 border border-emerald-200 dark:border-emerald-500/30">
        <span className="h-1.5 w-1.5 rounded-full bg-emerald-500" />
        GRAVANDO
      </span>
    );
  }
  if (ch.is_recording === false) {
    return (
      <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-semibold bg-rose-50 dark:bg-rose-500/15 text-rose-700 dark:text-rose-300 border border-rose-200 dark:border-rose-500/30">
        <span className="h-1.5 w-1.5 rounded-full bg-rose-500" />
        SEM GRAVAÇÃO
      </span>
    );
  }
  // Nunca afirmar "não gravou" sem ter conseguido perguntar.
  return (
    <span
      className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-semibold bg-slate-100 dark:bg-slate-800 text-slate-600 dark:text-slate-400 border border-slate-200 dark:border-slate-700"
      title={ch.error || 'Não foi possível determinar'}
    >
      DESCONHECIDO
    </span>
  );
};

export const RecordingsPage: React.FC<RecordingsPageProps> = ({ cameras }) => {
  const [result, setResult] = useState<RecordingCheckResult | null>(null);
  const [isChecking, setIsChecking] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [period, setPeriod] = useState<PeriodPreset>(24);
  const [searchQuery, setSearchQuery] = useState('');
  const [statusFilter, setStatusFilter] = useState<StatusFilter>('all');
  const [copied, setCopied] = useState(false);

  const recorders = useMemo(
    () => cameras.filter((c) => c.device_type === 'nvr' || c.device_type === 'dvr'),
    [cameras],
  );

  /**
   * Disparo manual, de propósito: cada verificação consulta o subsistema de disco
   * de todos os NVRs do site. Auto-refresh martelaria os gravadores por uma ação
   * que o técnico faz deliberadamente.
   */
  const runCheck = async () => {
    setIsChecking(true);
    setError(null);
    try {
      const end = new Date();
      const start = new Date(end.getTime() - period * 3600 * 1000);
      setResult(await api.checkRecordings(start.toISOString(), end.toISOString()));
    } catch (e: any) {
      setError(e?.toString() || 'Falha ao consultar os gravadores');
    } finally {
      setIsChecking(false);
    }
  };

  const rows: Row[] = useMemo(() => {
    if (!result) return [];
    return result.nvr_reports.flatMap((r) => [
      ...r.channels.map((ch) => ({
        nvrName: r.nvr_name,
        nvrHost: r.nvr_host,
        registered: true,
        ch,
      })),
      ...r.unregistered_channels.map((ch) => ({
        nvrName: r.nvr_name,
        nvrHost: r.nvr_host,
        registered: false,
        ch,
      })),
    ]);
  }, [result]);

  const filteredRows = useMemo(() => {
    const q = searchQuery.toLowerCase().trim();
    return rows.filter((row) => {
      if (q) {
        const hay = [
          row.ch.matched_camera_name || '',
          row.ch.channel_name,
          row.nvrName,
          row.nvrHost,
          row.ch.ip_address || '',
          String(row.ch.channel_id),
        ]
          .join(' ')
          .toLowerCase();
        if (!hay.includes(q)) return false;
      }
      switch (statusFilter) {
        case 'recording':
          return row.ch.is_recording === true;
        case 'not_recording':
          return row.ch.is_recording === false;
        case 'unregistered':
          return !row.registered;
        case 'unknown':
          return row.ch.is_recording === null || row.ch.is_recording === undefined;
        default:
          return true;
      }
    });
  }, [rows, searchQuery, statusFilter]);

  const failedNvrs = result?.nvr_reports.filter((r) => !r.reachable || !r.auth_ok) ?? [];

  const counts = useMemo(
    () => ({
      all: rows.length,
      recording: rows.filter((r) => r.ch.is_recording === true).length,
      notRecording: rows.filter((r) => r.ch.is_recording === false).length,
      unregistered: rows.filter((r) => !r.registered).length,
      unknown: rows.filter((r) => r.ch.is_recording === null || r.ch.is_recording === undefined)
        .length,
    }),
    [rows],
  );

  /** CSV das linhas atualmente filtradas — o que está na tela é o que se exporta. */
  const buildCsv = (): string => {
    const esc = (v: string) => `"${v.replace(/"/g, '""')}"`;
    const header = [
      'Camera',
      'Canal',
      'Nome do Canal',
      'NVR',
      'IP do NVR',
      'IP do Canal',
      'Canal Online',
      'Gravando',
      'Cobertura',
      'Ultima Gravacao',
      'Observacao',
    ];
    const lines = filteredRows.map((row) => {
      const last = row.ch.segments[row.ch.segments.length - 1];
      return [
        row.ch.matched_camera_name || (row.registered ? '' : 'NAO CADASTRADA'),
        String(row.ch.channel_id),
        row.ch.channel_name,
        row.nvrName,
        row.nvrHost,
        row.ch.ip_address || '',
        row.ch.online === null || row.ch.online === undefined
          ? 'desconhecido'
          : row.ch.online
            ? 'sim'
            : 'nao',
        row.ch.is_recording === null || row.ch.is_recording === undefined
          ? 'desconhecido'
          : row.ch.is_recording
            ? 'sim'
            : 'nao',
        pct(row.ch.coverage_ratio),
        last ? `${fmtTime(last.start)} - ${fmtTime(last.end)}` : '',
        row.ch.error || (row.ch.truncated ? 'amostra parcial' : ''),
      ]
        .map(esc)
        .join(',');
    });
    return [header.map(esc).join(','), ...lines].join('\n');
  };

  const handleExportCsv = () => {
    // BOM para o Excel abrir os acentos corretamente.
    const blob = new Blob(['﻿' + buildCsv()], { type: 'text/csv;charset=utf-8;' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `gravacoes-${new Date().toISOString().slice(0, 16).replace(/[:T]/g, '-')}.csv`;
    a.click();
    URL.revokeObjectURL(url);
  };

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(buildCsv());
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (e) {
      console.error('[Gravacoes] Falha ao copiar para a area de transferencia', e);
    }
  };

  return (
    <div className="flex flex-col h-full select-none">
      {/* Cabeçalho + controles */}
      <div className="px-6 pt-5 pb-3 space-y-3 shrink-0">
        <div className="flex items-start justify-between gap-4 flex-wrap">
          <div>
            <h3 className="text-xl font-bold text-slate-900 dark:text-white tracking-tight flex items-center gap-2">
              <Film className="h-5 w-5 text-sky-500" />
              Verificação de Gravações
            </h3>
            <p className="text-xs text-slate-500 dark:text-slate-400 mt-1">
              Consulta ao vivo os NVRs cadastrados e confere, canal a canal, se as câmeras estão
              realmente gravando.
            </p>
          </div>

          <div className="flex items-center gap-2 flex-wrap">
            <div className="flex items-center gap-1 bg-slate-100 dark:bg-slate-950 border border-slate-200 dark:border-slate-800 rounded-lg p-0.5">
              {([6, 24, 72] as PeriodPreset[]).map((p) => (
                <button
                  key={p}
                  onClick={() => setPeriod(p)}
                  className={`px-2.5 py-1 rounded text-xs font-semibold transition ${
                    period === p
                      ? 'bg-sky-600 text-white shadow-sm'
                      : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white'
                  }`}
                >
                  {p}h
                </button>
              ))}
            </div>

            <button
              onClick={runCheck}
              disabled={isChecking || recorders.length === 0}
              className="px-3.5 py-1.5 rounded-lg bg-sky-600 hover:bg-sky-500 disabled:opacity-50 disabled:cursor-not-allowed text-white text-xs font-bold flex items-center gap-1.5 shadow-md transition"
            >
              {isChecking ? (
                <Loader2 className="h-3.5 w-3.5 animate-spin" />
              ) : (
                <RefreshCw className="h-3.5 w-3.5" />
              )}
              {isChecking ? `Consultando ${recorders.length} NVR(s)…` : 'Verificar Agora'}
            </button>

            {result && (
              <>
                <button
                  onClick={handleExportCsv}
                  className="px-3 py-1.5 rounded-lg bg-slate-100 dark:bg-slate-800 hover:bg-slate-200 dark:hover:bg-slate-700 border border-slate-200 dark:border-slate-700 text-slate-700 dark:text-slate-200 text-xs font-semibold flex items-center gap-1.5 transition"
                >
                  <Download className="h-3.5 w-3.5" />
                  CSV
                </button>
                <button
                  onClick={handleCopy}
                  className="px-3 py-1.5 rounded-lg bg-slate-100 dark:bg-slate-800 hover:bg-slate-200 dark:hover:bg-slate-700 border border-slate-200 dark:border-slate-700 text-slate-700 dark:text-slate-200 text-xs font-semibold flex items-center gap-1.5 transition"
                >
                  <Copy className="h-3.5 w-3.5" />
                  {copied ? 'Copiado!' : 'Copiar'}
                </button>
              </>
            )}
          </div>
        </div>

        {result && (
          <div className="flex items-center gap-3 flex-wrap">
            <div className="relative flex-1 min-w-[240px] max-w-md">
              <Search className="h-3.5 w-3.5 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2" />
              <input
                type="text"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder="Buscar por câmera, canal, NVR ou IP..."
                className="w-full bg-slate-50 dark:bg-slate-950 border border-slate-200 dark:border-slate-800 rounded-lg pl-9 pr-3 py-1.5 text-xs text-slate-800 dark:text-slate-200 focus:outline-none focus:border-sky-500 transition"
              />
            </div>

            <div className="flex items-center gap-1 bg-slate-100 dark:bg-slate-950 border border-slate-200 dark:border-slate-800 rounded-lg p-0.5">
              {(
                [
                  ['all', `Todos (${counts.all})`],
                  ['recording', `Gravando (${counts.recording})`],
                  ['not_recording', `Sem gravação (${counts.notRecording})`],
                  ['unregistered', `Não cadastrada (${counts.unregistered})`],
                  ['unknown', `Desconhecido (${counts.unknown})`],
                ] as [StatusFilter, string][]
              ).map(([id, label]) => (
                <button
                  key={id}
                  onClick={() => setStatusFilter(id)}
                  className={`px-2.5 py-1 rounded text-xs font-semibold transition ${
                    statusFilter === id
                      ? 'bg-sky-600 text-white shadow-sm'
                      : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white'
                  }`}
                >
                  {label}
                </button>
              ))}
            </div>

            <span className="text-[11px] font-mono text-slate-500 dark:text-slate-400">
              {fmtTime(result.period_start)} → {fmtTime(result.period_end)}
            </span>
          </div>
        )}

        {error && (
          <div className="flex items-center gap-2 px-3 py-2 rounded-lg bg-rose-50 dark:bg-rose-500/10 border border-rose-200 dark:border-rose-500/30 text-xs text-rose-700 dark:text-rose-300">
            <AlertTriangle className="h-4 w-4 shrink-0" />
            {error}
          </div>
        )}

        {/* Um aviso por gravador com falha, em vez de N linhas de erro por canal. */}
        {failedNvrs.map((r) => (
          <div
            key={r.nvr_id}
            className="flex items-center gap-2 px-3 py-2 rounded-lg bg-amber-50 dark:bg-amber-500/10 border border-amber-200 dark:border-amber-500/30 text-xs text-amber-800 dark:text-amber-200"
          >
            {r.auth_ok ? (
              <ServerCrash className="h-4 w-4 shrink-0" />
            ) : (
              <KeyRound className="h-4 w-4 shrink-0" />
            )}
            <span className="font-semibold">{r.nvr_name}</span>
            <span className="font-mono text-[11px] opacity-80">({r.nvr_host})</span>
            <span>
              —{' '}
              {!r.auth_ok
                ? 'falha de autenticação: verifique usuário e senha do gravador'
                : `inacessível: ${r.error || 'sem resposta'}`}
            </span>
          </div>
        ))}
      </div>

      {/* Conteúdo */}
      <div className="flex-1 min-h-0 overflow-auto px-6 pb-6">
        {recorders.length === 0 ? (
          <div className="h-full flex items-center justify-center p-12 text-center">
            <div className="space-y-3 max-w-md">
              <div className="h-12 w-12 rounded-2xl bg-sky-50 dark:bg-sky-500/15 text-sky-600 dark:text-sky-400 flex items-center justify-center mx-auto">
                <HardDriveDownload className="h-6 w-6" />
              </div>
              <h4 className="text-base font-bold text-slate-800 dark:text-white">
                Nenhum NVR cadastrado
              </h4>
              <p className="text-xs text-slate-500 dark:text-slate-400">
                Cadastre o gravador em <strong>Dispositivos Cadastrados</strong>, marcando o Tipo como
                NVR ou DVR e informando a porta HTTP correta (normalmente 80, 8000 ou 8080).
              </p>
            </div>
          </div>
        ) : !result ? (
          <div className="h-full flex items-center justify-center p-12 text-center">
            <div className="space-y-3 max-w-md">
              <div className="h-12 w-12 rounded-2xl bg-slate-100 dark:bg-slate-800 text-slate-500 dark:text-slate-400 flex items-center justify-center mx-auto">
                <Film className="h-6 w-6" />
              </div>
              <h4 className="text-base font-bold text-slate-800 dark:text-white">
                {recorders.length} gravador(es) pronto(s) para consulta
              </h4>
              <p className="text-xs text-slate-500 dark:text-slate-400">
                Clique em <strong>Verificar Agora</strong> para consultar as gravações das últimas{' '}
                {period}h. A consulta é feita ao vivo nos gravadores e pode levar alguns segundos.
              </p>
            </div>
          </div>
        ) : filteredRows.length === 0 ? (
          <div className="h-full flex items-center justify-center p-12 text-center text-xs text-slate-500 dark:text-slate-400">
            {rows.length === 0
              ? 'Os gravadores responderam, mas nenhum canal foi retornado.'
              : 'Nenhum canal encontrado com os filtros selecionados.'}
          </div>
        ) : (
          <div className="space-y-6">
            <div className="border border-slate-200 dark:border-slate-800 rounded-xl overflow-hidden shadow-sm">
              <table className="w-full text-left text-xs font-mono border-collapse">
                <thead className="bg-slate-50 dark:bg-slate-900 text-slate-600 dark:text-slate-400 uppercase text-[10px] tracking-wider border-b border-slate-200 dark:border-slate-800 sticky top-0 backdrop-blur z-10">
                  <tr>
                    <th className="px-3 py-2.5">Câmera</th>
                    <th className="px-3 py-2.5">Canal</th>
                    <th className="px-3 py-2.5">NVR</th>
                    <th className="px-3 py-2.5">IP do Canal</th>
                    <th className="px-3 py-2.5">Canal</th>
                    <th className="px-3 py-2.5">Gravando</th>
                    <th className="px-3 py-2.5">Cobertura</th>
                    <th className="px-3 py-2.5">Última Gravação</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-slate-100 dark:divide-slate-800/60 bg-white dark:bg-slate-950">
                  {filteredRows.map((row) => {
                    const last = row.ch.segments[row.ch.segments.length - 1];
                    const analog = !row.ch.ip_address;
                    return (
                      <tr
                        key={`${row.nvrHost}-${row.ch.channel_id}`}
                        className="hover:bg-sky-50/60 dark:hover:bg-slate-900/60 transition"
                      >
                        <td className="px-3 py-2">
                          {row.registered ? (
                            <span className="font-sans font-semibold text-slate-800 dark:text-slate-100">
                              {row.ch.matched_camera_name}
                            </span>
                          ) : analog ? (
                            <span className="italic text-slate-500 dark:text-slate-400">
                              canal analógico/local
                            </span>
                          ) : (
                            <span className="italic text-amber-600 dark:text-amber-400">
                              — não cadastrada —
                            </span>
                          )}
                        </td>
                        <td className="px-3 py-2 text-slate-600 dark:text-slate-300">
                          <span className="font-bold">{row.ch.channel_id}</span>
                          {row.ch.channel_name && (
                            <span className="ml-1.5 text-slate-400 dark:text-slate-500">
                              {row.ch.channel_name}
                            </span>
                          )}
                        </td>
                        <td className="px-3 py-2 text-slate-600 dark:text-slate-300">
                          <span className="font-sans">{row.nvrName}</span>
                          <span className="ml-1.5 text-slate-400 dark:text-slate-500">
                            {row.nvrHost}
                          </span>
                        </td>
                        <td className="px-3 py-2 text-slate-500 dark:text-slate-400">
                          {row.ch.ip_address || '—'}
                        </td>
                        <td className="px-3 py-2">
                          {row.ch.online === true ? (
                            <span className="text-emerald-600 dark:text-emerald-400">online</span>
                          ) : row.ch.online === false ? (
                            <span className="text-rose-600 dark:text-rose-400">offline</span>
                          ) : (
                            <span className="text-slate-400 dark:text-slate-500">—</span>
                          )}
                        </td>
                        <td className="px-3 py-2">
                          <RecordingBadge ch={row.ch} />
                          {row.ch.error && (
                            <div
                              className="text-[10px] text-slate-400 dark:text-slate-500 mt-0.5 max-w-[220px] truncate"
                              title={row.ch.error}
                            >
                              {row.ch.error}
                            </div>
                          )}
                        </td>
                        <td className="px-3 py-2">
                          <CoverageBar
                            segments={row.ch.segments}
                            periodStart={result.period_start}
                            periodEnd={result.period_end}
                            ratio={row.ch.coverage_ratio}
                            truncated={row.ch.truncated}
                          />
                        </td>
                        <td className="px-3 py-2 text-slate-500 dark:text-slate-400 text-[11px]">
                          {last ? `${fmtTime(last.start)} → ${fmtTime(last.end)}` : '—'}
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>

            {/* Câmeras cadastradas que não estão em nenhum NVR: descompasso de
                natureza diferente, por isso em tabela própria. */}
            {result.orphan_cameras.length > 0 && (
              <div>
                <h4 className="text-xs font-bold text-slate-700 dark:text-slate-200 uppercase tracking-wider mb-2 flex items-center gap-1.5">
                  <AlertTriangle className="h-3.5 w-3.5 text-amber-500" />
                  Câmeras cadastradas fora de qualquer NVR ({result.orphan_cameras.length})
                </h4>
                <div className="border border-amber-200 dark:border-amber-500/30 rounded-xl overflow-hidden">
                  <table className="w-full text-left text-xs font-mono border-collapse">
                    <thead className="bg-amber-50 dark:bg-amber-500/10 text-amber-800 dark:text-amber-300 uppercase text-[10px] tracking-wider">
                      <tr>
                        <th className="px-3 py-2">Câmera</th>
                        <th className="px-3 py-2">IP</th>
                        <th className="px-3 py-2">Observação</th>
                      </tr>
                    </thead>
                    <tbody className="divide-y divide-amber-100 dark:divide-amber-500/20 bg-white dark:bg-slate-950">
                      {result.orphan_cameras.map((c) => (
                        <tr key={c.id}>
                          <td className="px-3 py-2 font-sans font-semibold text-slate-800 dark:text-slate-100">
                            {c.name}
                          </td>
                          <td className="px-3 py-2 text-slate-500 dark:text-slate-400">{c.host}</td>
                          <td className="px-3 py-2 text-slate-500 dark:text-slate-400 font-sans">
                            Não encontrada em nenhum gravador consultado — pode não estar sendo
                            gravada.
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
};
