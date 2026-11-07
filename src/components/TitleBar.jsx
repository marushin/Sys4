import { useEffect, useState } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api/core';
import { getConfigValue, setConfigValue, loadAppConfig } from '../utils/config';
import { KEY_THEME, applyTheme, isDarkActive } from '../utils/theme';

const KEY_ALWAYS_ON_TOP = 'sys4_always_on_top';

function TitleBar() {
    const appWindow = getCurrentWindow();
    const [isAlwaysOnTop, setIsAlwaysOnTop] = useState(() => getConfigValue(KEY_ALWAYS_ON_TOP) === 'true');

    // Restore theme and always-on-top from the persisted config.
    useEffect(() => {
        loadAppConfig().then((config) => {
            if (config[KEY_THEME]) applyTheme(config[KEY_THEME]);
            const aot = config[KEY_ALWAYS_ON_TOP] === 'true';
            setIsAlwaysOnTop(aot);
            if (aot) {
                getCurrentWindow()
                    .setAlwaysOnTop(true)
                    .catch((e) => console.error('Failed to set always on top:', e));
            }
        });
    }, []);

    const toggleTheme = () => {
        const next = isDarkActive() ? 'light' : 'dark';
        applyTheme(next);
        setConfigValue(KEY_THEME, next);
    };

    const toggleAlwaysOnTop = async () => {
        const next = !isAlwaysOnTop;
        try {
            await appWindow.setAlwaysOnTop(next);
            setIsAlwaysOnTop(next);
            setConfigValue(KEY_ALWAYS_ON_TOP, String(next));
        } catch (e) {
            console.error('setAlwaysOnTop failed:', e);
        }
    };

    const toggleWidget = () => {
        invoke('toggle_widget').catch((e) => console.error('Toggle widget failed:', e));
    };

    return (
        <div className="titlebar">
            <div className="titlebar-label" data-tauri-drag-region>
                <img src="/app_icon.png" alt="" className="titlebar-icon" data-tauri-drag-region />
                <button
                    type="button"
                    className="titlebar-brand"
                    onClick={toggleTheme}
                    title="다크/라이트 테마 전환"
                    aria-label="다크/라이트 테마 전환"
                >
                    Sys4
                </button>
            </div>

            <div className="titlebar-spacer" data-tauri-drag-region />

            <div className="titlebar-actions">
                <button type="button" className="titlebar-btn" onClick={toggleWidget} title="미니 위젯으로 전환" aria-label="미니 위젯으로 전환">
                    🖥️
                </button>
                <button
                    type="button"
                    className={`titlebar-btn pin-btn ${isAlwaysOnTop ? 'active' : ''}`}
                    onClick={toggleAlwaysOnTop}
                    title={isAlwaysOnTop ? '항상 위 해제' : '항상 위 설정'}
                    aria-label="항상 위"
                    aria-pressed={isAlwaysOnTop}
                >
                    📌
                </button>
                <button
                    type="button"
                    className="titlebar-btn"
                    onClick={() => appWindow.minimize().catch(console.error)}
                    title="최소화"
                    aria-label="최소화"
                >
                    ─
                </button>
                <button
                    type="button"
                    className="titlebar-btn close"
                    onClick={() => appWindow.close().catch(console.error)}
                    title="종료"
                    aria-label="종료"
                >
                    ✕
                </button>
            </div>
        </div>
    );
}

export default TitleBar;
