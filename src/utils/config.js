import { invoke } from '@tauri-apps/api/core';

// Settings are persisted by the Rust backend (config.json) and mirrored into localStorage so
// synchronous reads during the first render (theme, pin state) are instant.
//
// Writes are sent as *partial* updates that the backend merges, so the main window and the
// widget can never overwrite each other's keys with a stale copy.

let memoryConfig = {};
let loadPromise = null;

function mirrorToLocalStorage(entries) {
    try {
        for (const [k, v] of Object.entries(entries)) localStorage.setItem(k, v);
    } catch {
        // storage unavailable — the backend copy is authoritative anyway
    }
}

/** Load config once per window (subsequent calls reuse the same promise). */
export function loadAppConfig() {
    if (!loadPromise) {
        loadPromise = invoke('get_app_config')
            .then((config) => {
                memoryConfig = config || {};
                mirrorToLocalStorage(memoryConfig);
                return memoryConfig;
            })
            .catch((e) => {
                console.error('Failed to load app config:', e);
                return memoryConfig;
            });
    }
    return loadPromise;
}

export function getConfigValue(key, defaultValue = null) {
    if (memoryConfig[key] !== undefined) return memoryConfig[key];
    try {
        const local = localStorage.getItem(key);
        if (local !== null) return local;
    } catch {
        // ignore
    }
    return defaultValue;
}

/** Persist one or more keys: `setConfigValues({ a: 1, b: 'x' })`. */
export async function setConfigValues(values) {
    const entries = Object.fromEntries(Object.entries(values).map(([k, v]) => [k, String(v)]));
    Object.assign(memoryConfig, entries);
    mirrorToLocalStorage(entries);
    try {
        await invoke('update_app_config', { values: entries });
    } catch (e) {
        console.error('Failed to save app config:', e);
    }
}

export function setConfigValue(key, value) {
    return setConfigValues({ [key]: value });
}
