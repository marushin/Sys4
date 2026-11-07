import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

export const POLLING_INTERVAL = 2000;

/**
 * Poll `get_system_resources` while the window is visible.
 *
 * - Uses a self-rescheduling timeout, so a slow backend call can never overlap the next one.
 * - Skips the backend call while the window is hidden/minimized and refreshes immediately on return.
 * - `error` is true while the latest request failed (the previous data is kept as stale).
 */
export function useSystemResources(options, interval = POLLING_INTERVAL) {
    const [data, setData] = useState(null);
    const [error, setError] = useState(false);
    const optionsKey = JSON.stringify(options);

    useEffect(() => {
        const opts = JSON.parse(optionsKey);
        let timer = null;
        let inFlight = false;
        let cancelled = false;

        const schedule = () => {
            if (!cancelled) timer = setTimeout(tick, interval);
        };

        const tick = async () => {
            timer = null;
            if (cancelled) return;
            // Hidden: keep a cheap heartbeat (no IPC) in case a visibilitychange is missed.
            if (document.visibilityState !== 'visible') {
                schedule();
                return;
            }

            inFlight = true;
            try {
                const resources = await invoke('get_system_resources', { options: opts });
                if (!cancelled) {
                    setData(resources);
                    setError(false);
                }
            } catch (e) {
                console.error('Failed to get system resources:', e);
                if (!cancelled) setError(true);
            } finally {
                inFlight = false;
            }

            schedule();
        };

        // Refresh immediately when the window becomes visible again.
        const onVisibilityChange = () => {
            if (document.visibilityState !== 'visible' || inFlight) return;
            clearTimeout(timer);
            tick();
        };

        tick();
        document.addEventListener('visibilitychange', onVisibilityChange);
        return () => {
            cancelled = true;
            if (timer) clearTimeout(timer);
            document.removeEventListener('visibilitychange', onVisibilityChange);
        };
    }, [optionsKey, interval]);

    return { data, error };
}
