import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
    usageLevel,
    temperatureLevel,
    clampPercent,
    formatSpeed,
    formatSpeedCompact,
    formatUptime,
    driveLetter,
} from '../utils/format';
import { useGpuCache, useLastDefined } from '../hooks/useGpuCache';
import { useFitWindowToContent } from '../hooks/useWindowBehavior';

const WINDOW_WIDTH = 260;
// Title bar (36) + main padding (2 × 10) + subpixel safety buffer (2)
const CHROME_HEIGHT = 36 + 20 + 2;
const TOAST_MS = 3000;

/** Collapsible card. The header is keyboard accessible (Enter / Space). */
function Section({ id, title, expanded, onToggle, summary, actions, children }) {
    const detailsId = `section-${id}`;
    const onKeyDown = (e) => {
        // Ignore keys coming from nested controls (e.g. the RAM clean button).
        if (e.target !== e.currentTarget) return;
        if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault();
            onToggle(id);
        }
    };

    return (
        <section className="resource-section">
            <div
                className={`resource-section-title collapsible ${expanded ? 'expanded' : ''}`}
                role="button"
                tabIndex={0}
                aria-expanded={expanded}
                aria-controls={detailsId}
                onClick={() => onToggle(id)}
                onKeyDown={onKeyDown}
            >
                <div className="section-title-left">
                    <span>{title}</span>
                    {actions}
                </div>
                <div className="section-title-right">
                    {!expanded && summary}
                    <span className="chevron" aria-hidden="true">▼</span>
                </div>
            </div>
            {expanded && (
                <div className="resource-section-details" id={detailsId}>
                    {children}
                </div>
            )}
        </section>
    );
}

function Meter({ value, kind, label }) {
    const level = usageLevel(value);
    return (
        <div
            className="resource-progress"
            role="progressbar"
            aria-valuenow={clampPercent(value)}
            aria-valuemin="0"
            aria-valuemax="100"
            aria-label={label}
        >
            <div
                className={`resource-progress-bar ${kind} ${level}`}
                style={{ width: `${clampPercent(value)}%` }}
            />
        </div>
    );
}

function Row({ label, value, level = '', labelClass = '', valueClass = '', title }) {
    return (
        <div className="resource-row">
            <span className={`resource-label ${labelClass}`} title={title}>{label}</span>
            <span className={`resource-value ${level} ${valueClass}`}>{value}</span>
        </div>
    );
}

function SubItem({ label, value, level = '' }) {
    return (
        <div className="sub-item">
            <span className="resource-label">{label}</span>
            <span className={`resource-value ${level}`}>{value}</span>
        </div>
    );
}

function Summary({ value, prefix = '' }) {
    if (value == null) return <span className="section-summary muted">—</span>;
    return <span className={`section-summary ${usageLevel(value)}`}>{prefix}{value}%</span>;
}

const gb = (v) => (v != null ? `${v} GB` : '—');
const pct = (v) => (v != null ? `${v}%` : '—');

function ResourcePanel({ systemResources: r, stale, expanded, onToggleExpanded }) {
    const containerRef = useRef(null);
    const [isCleaning, setIsCleaning] = useState(false);
    const [cleanResult, setCleanResult] = useState(null);
    const { hasGpu, gpu } = useGpuCache(r);

    // Keep the last good network/disk sample so sections don't flash empty.
    const network = useLastDefined(r?.network);
    const disks = useLastDefined(r?.disks?.length ? r.disks : null);

    useFitWindowToContent(containerRef, WINDOW_WIDTH, CHROME_HEIGHT);

    useEffect(() => {
        if (!cleanResult) return undefined;
        const t = setTimeout(() => setCleanResult(null), TOAST_MS);
        return () => clearTimeout(t);
    }, [cleanResult]);

    const handleClearRam = async (e) => {
        e.stopPropagation();
        if (isCleaning) return;
        setIsCleaning(true);
        setCleanResult(null);
        try {
            setCleanResult(await invoke('clear_ram_cache'));
        } catch (err) {
            setCleanResult(`정리 실패: ${err}`);
        } finally {
            setIsCleaning(false);
        }
    };

    const sectionProps = (id) => ({ id, expanded: expanded[id], onToggle: onToggleExpanded });

    const diskSummary = (() => {
        if (!disks?.length) return null;
        const shown = disks.length <= 2
            ? disks
            : [disks.find((d) => d.name.toUpperCase().startsWith('C:')) || disks[0]];
        return (
            <span className="section-summary-group">
                {shown.map((d) => (
                    <span key={d.name} className={`section-summary ${usageLevel(d.usage_percent)}`} title={`${d.name}: ${d.usage_percent}%`}>
                        {driveLetter(d.name)} {d.usage_percent}%
                    </span>
                ))}
                {disks.length > 2 && <span className="section-summary-sub">(+{disks.length - 1})</span>}
            </span>
        );
    })();

    return (
        <div className="resource-content" ref={containerRef}>
            {stale && (
                <div className="stale-banner" role="status">
                    데이터 갱신 실패 · 재시도 중…
                </div>
            )}

            {/* CPU */}
            <Section {...sectionProps('cpu')} title="CPU" summary={r && <Summary value={r.cpu_usage_percent} />}>
                {r && (
                    <>
                        {r.cpu_name && <div className="resource-device-name" title={r.cpu_name}>{r.cpu_name}</div>}
                        <Row label="사용률" value={pct(r.cpu_usage_percent)} level={usageLevel(r.cpu_usage_percent)} />
                        <Meter value={r.cpu_usage_percent} kind="cpu" label="CPU 사용률" />
                        <div className="resource-row sub-grid">
                            <SubItem label="코어" value={`${r.cpu_cores}`} />
                            <SubItem label="가동" value={formatUptime(r.uptime_secs)} />
                        </div>
                    </>
                )}
            </Section>

            {/* RAM */}
            <Section
                {...sectionProps('ram')}
                title="RAM"
                summary={r && <Summary value={r.ram_usage_percent} />}
                actions={
                    <>
                        <button
                            type="button"
                            className={`ram-clean-btn ${isCleaning ? 'cleaning' : ''}`}
                            onClick={handleClearRam}
                            onKeyDown={(e) => e.stopPropagation()}
                            disabled={isCleaning}
                            title="RAM 캐시 정리"
                            aria-label="RAM 캐시 정리"
                        >
                            {isCleaning ? '···' : '🧹'}
                        </button>
                        {cleanResult && <span className="clean-result-toast" role="status">{cleanResult}</span>}
                    </>
                }
            >
                {r && (
                    <>
                        <Row label="사용률" value={pct(r.ram_usage_percent)} level={usageLevel(r.ram_usage_percent)} />
                        <Meter value={r.ram_usage_percent} kind="ram" label="RAM 사용률" />
                        <div className="resource-row sub-grid">
                            <SubItem label="사용" value={gb(r.ram_used_gb)} />
                            <SubItem label="전체" value={gb(r.ram_total_gb)} />
                            {r.swap_total_gb > 0 && (
                                <>
                                    <SubItem label="가상 사용" value={gb(r.swap_used_gb)} />
                                    <SubItem label="가상 전체" value={gb(r.swap_total_gb)} />
                                </>
                            )}
                        </div>
                    </>
                )}
            </Section>

            {/* GPU */}
            {hasGpu && (
                <Section
                    {...sectionProps('gpu')}
                    title="GPU"
                    summary={gpu && (gpu.utilization_percent != null
                        ? <Summary value={gpu.utilization_percent} />
                        : <Summary value={gpu.memory_usage_percent} prefix="VRAM " />)}
                >
                    {gpu && (
                        <>
                            <div className="resource-device-name" title={gpu.name}>{gpu.name}</div>
                            <Row label="사용률" value={pct(gpu.utilization_percent)} level={usageLevel(gpu.utilization_percent)} />
                            <Row label="VRAM" value={pct(gpu.memory_usage_percent)} level={usageLevel(gpu.memory_usage_percent)} />
                            <Meter value={gpu.memory_usage_percent} kind="gpu" label="GPU 메모리 사용률" />
                            <div className="resource-row sub-grid">
                                <SubItem label="사용" value={gb(gpu.memory_used_gb)} />
                                <SubItem label="전체" value={gb(gpu.memory_total_gb)} />
                                {gpu.temperature_c != null && (
                                    <SubItem label="온도" value={`${gpu.temperature_c}°C`} level={temperatureLevel(gpu.temperature_c)} />
                                )}
                                {gpu.power_w != null && <SubItem label="전력" value={`${gpu.power_w} W`} />}
                            </div>
                        </>
                    )}
                </Section>
            )}

            {/* Network */}
            <Section
                {...sectionProps('network')}
                title="네트워크"
                summary={network && (
                    <span className="section-summary-net">
                        <span className="net-down-val">↓ {formatSpeedCompact(network.download_kbps)}</span>
                        <span className="net-up-val">↑ {formatSpeedCompact(network.upload_kbps)}</span>
                    </span>
                )}
            >
                <Row label="↓ 다운" labelClass="net-down" valueClass="net-down-val" value={formatSpeed(network?.download_kbps)} />
                <Row label="↑ 업" labelClass="net-up" valueClass="net-up-val" value={formatSpeed(network?.upload_kbps)} />
                <div className="resource-row sub-grid">
                    <SubItem label="총 수신" value={gb(network?.total_received_gb)} />
                    <SubItem label="총 송신" value={gb(network?.total_transmitted_gb)} />
                </div>
            </Section>

            {/* Disk */}
            <Section {...sectionProps('disk')} title="디스크" summary={diskSummary}>
                {disks?.length ? (
                    disks.map((d) => (
                        <div key={d.name} className="disk-item">
                            <Row label={d.name} title={d.name} labelClass="disk-name" value={`${d.usage_percent}%`} level={usageLevel(d.usage_percent)} />
                            <Meter value={d.usage_percent} kind="disk" label={`${d.name} 사용률`} />
                            <div className="resource-row sub-grid">
                                <SubItem label="사용" value={gb(d.used_gb)} />
                                <SubItem label="전체" value={gb(d.total_gb)} />
                            </div>
                        </div>
                    ))
                ) : (
                    <div className="resource-placeholder">디스크 정보 조회 중…</div>
                )}
            </Section>
        </div>
    );
}

export default ResourcePanel;
