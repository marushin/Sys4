import { useEffect, useState } from 'react';
import { getVersion } from '@tauri-apps/api/app';
import { openUrl } from '@tauri-apps/plugin-opener';
import { compareVersions } from '../utils/format';
import { getConfigValue, setConfigValue, loadAppConfig } from '../utils/config';

const RELEASES_API = 'https://api.github.com/repos/marushin/Sys4/releases/latest';
const KEY_DISMISSED = 'sys4_dismissed_update';

function CheckUpdate() {
    const [update, setUpdate] = useState(null);

    useEffect(() => {
        const controller = new AbortController();

        (async () => {
            try {
                const [current] = await Promise.all([getVersion(), loadAppConfig()]);
                const res = await fetch(RELEASES_API, {
                    signal: controller.signal,
                    headers: { Accept: 'application/vnd.github+json' },
                });
                if (!res.ok) return;
                const data = await res.json();
                if (data.draft || data.prerelease || !data.tag_name) return;

                const latest = data.tag_name.replace(/^v/i, '');
                if (compareVersions(latest, current) > 0 && getConfigValue(KEY_DISMISSED) !== latest) {
                    setUpdate({ version: latest, url: data.html_url });
                }
            } catch (e) {
                if (e.name !== 'AbortError') console.error('Failed to check for updates:', e);
            }
        })();

        return () => controller.abort();
    }, []);

    if (!update) return null;

    const openRelease = () => openUrl(update.url).catch((e) => console.error('Failed to open URL:', e));
    const dismiss = () => {
        setConfigValue(KEY_DISMISSED, update.version);
        setUpdate(null);
    };

    return (
        <div className="update-notification" role="status">
            <span className="update-text">
                새 버전 <strong>v{update.version}</strong>
            </span>
            <div className="update-actions">
                <button type="button" onClick={openRelease} className="update-button">
                    받기
                </button>
                <button type="button" onClick={dismiss} className="update-close" title="이 버전 알림 숨기기" aria-label="이 버전 알림 숨기기">
                    ✕
                </button>
            </div>
        </div>
    );
}

export default CheckUpdate;
