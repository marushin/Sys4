import { useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useSystemResources } from '../hooks/useSystemResources';
import { useGpuCache } from '../hooks/useGpuCache';
import { useFitWindowToContent, useSavePositionOnMove } from '../hooks/useWindowBehavior';
import { clampPercent, usageLevel } from '../utils/format';
import { applyStoredTheme } from '../utils/theme';

const WIDGET_WIDTH = 180;
const POLL_OPTIONS = { cpu: true, ram: true, gpu: true, network: false, disk: false };

function WidgetRow({ label, value, kind }) {
    return (
        <div className="widget-row">
            <span className="widget-label">{label}</span>
            <div className="widget-bar">
                <div className={`widget-bar-fill ${kind} ${usageLevel(value)}`} style={{ width: `${clampPercent(value)}%` }} />
            </div>
            <span className={`widget-value ${usageLevel(value)}`}>{value != null ? `${Math.round(value)}%` : '—'}</span>
        </div>
    );
}

function Widget() {
    const wrapperRef = useRef(null);
    const { data: r } = useSystemResources(POLL_OPTIONS);
    const { hasGpu, gpu } = useGpuCache(r);

    useSavePositionOnMove('widget_pos_x', 'widget_pos_y');
    useFitWindowToContent(wrapperRef, WIDGET_WIDTH);

    // The theme may have been changed in the main window while the widget was hidden.
    useEffect(() => {
        applyStoredTheme();
        const onVisible = () => document.visibilityState === 'visible' && applyStoredTheme();
        document.addEventListener('visibilitychange', onVisible);
        return () => document.removeEventListener('visibilitychange', onVisible);
    }, []);

    const showMain = (e) => {
        e?.stopPropagation();
        invoke('show_main_window').catch((err) => console.error('Failed to show main window:', err));
    };

    return (
        <div className="widget-wrapper" ref={wrapperRef}>
            <div className="widget-container" onClick={showMain} title="클릭하여 메인 창 열기">
                <div className="widget-drag-handle" data-tauri-drag-region onClick={(e) => e.stopPropagation()}>
                    <span data-tauri-drag-region>⋮⋮</span>
                </div>
                <button type="button" className="widget-close" onClick={showMain} title="위젯 닫기" aria-label="위젯 닫기">
                    ✕
                </button>

                {r ? (
                    <>
                        <WidgetRow label="CPU" value={r.cpu_usage_percent} kind="cpu" />
                        <WidgetRow label="RAM" value={r.ram_usage_percent} kind="ram" />
                        {hasGpu && <WidgetRow label="VRAM" value={gpu?.memory_usage_percent} kind="vram" />}
                    </>
                ) : (
                    <div className="widget-row">
                        <span className="widget-label">로딩…</span>
                    </div>
                )}
            </div>
        </div>
    );
}

export default Widget;
