import { useEffect, useLayoutEffect, useRef } from 'react';
import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';
import { setConfigValues } from '../utils/config';

/** Windows parks minimized windows at (-32000, -32000); never persist such coordinates. */
const INVALID_COORD = -10000;
const SAVE_DEBOUNCE_MS = 300;

/** Persist this window's position (debounced) whenever the user moves it. */
export function useSavePositionOnMove(keyX, keyY) {
    useEffect(() => {
        const win = getCurrentWindow();
        let timer = null;
        const unlistenPromise = win.onMoved(({ payload: pos }) => {
            if (pos.x <= INVALID_COORD || pos.y <= INVALID_COORD) return;
            clearTimeout(timer);
            timer = setTimeout(async () => {
                const minimized = await win.isMinimized().catch(() => false);
                if (!minimized) setConfigValues({ [keyX]: pos.x, [keyY]: pos.y });
            }, SAVE_DEBOUNCE_MS);
        });

        return () => {
            clearTimeout(timer);
            unlistenPromise.then((unlisten) => unlisten());
        };
    }, [keyX, keyY]);
}

/**
 * Resize the window to fit `ref`'s content. Only issues the IPC call when the computed
 * height actually changes (not on every 2-second data refresh).
 */
export function useFitWindowToContent(ref, width, extraHeight = 0) {
    const lastHeight = useRef(0);

    useLayoutEffect(() => {
        const el = ref.current;
        if (!el) return undefined;
        const win = getCurrentWindow();

        const fit = () => {
            const height = Math.ceil(el.scrollHeight + extraHeight);
            if (height <= 0 || height === lastHeight.current) return;
            lastHeight.current = height;
            win.setSize(new LogicalSize(width, height)).catch((e) =>
                console.error('Failed to resize window:', e),
            );
        };

        fit();
        const observer = new ResizeObserver(fit);
        observer.observe(el);
        return () => observer.disconnect();
    }, [ref, width, extraHeight]);
}
