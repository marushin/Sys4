// Pure formatting helpers (no Tauri imports) — unit-tested in format.test.js.

/** CSS class suffix for a usage percentage: '', 'warning' (>=75) or 'danger' (>=90). */
export function usageLevel(percent) {
    if (percent == null || Number.isNaN(percent)) return '';
    if (percent >= 90) return 'danger';
    if (percent >= 75) return 'warning';
    return '';
}

/** Same thresholds for temperatures in °C. */
export function temperatureLevel(celsius) {
    if (celsius == null) return '';
    if (celsius >= 90) return 'danger';
    if (celsius >= 75) return 'warning';
    return '';
}

/** Clamp to a 0–100 percentage usable as a bar width. */
export function clampPercent(percent) {
    const n = Number(percent);
    if (!Number.isFinite(n)) return 0;
    return Math.min(100, Math.max(0, n));
}

/** "512.0 KB/s" / "1.5 MB/s". */
export function formatSpeed(kbps) {
    if (kbps == null) return '—';
    if (kbps >= 1024 * 1024) return `${(kbps / (1024 * 1024)).toFixed(1)} GB/s`;
    if (kbps >= 1024) return `${(kbps / 1024).toFixed(1)} MB/s`;
    return `${kbps.toFixed(1)} KB/s`;
}

/** Compact form for collapsed headers: "1.2M/s", "450K/s", "3.4K/s", "0K/s". */
export function formatSpeedCompact(kbps) {
    if (kbps == null || kbps <= 0) return '0K/s';
    if (kbps >= 1024 * 1024) return `${(kbps / (1024 * 1024)).toFixed(1)}G/s`;
    if (kbps >= 1024) return `${(kbps / 1024).toFixed(1)}M/s`;
    if (kbps >= 100) return `${Math.round(kbps)}K/s`;
    return `${kbps.toFixed(1)}K/s`;
}

/** "3일 4시간", "5시간 12분", "7분". */
export function formatUptime(totalSecs) {
    if (totalSecs == null || totalSecs < 0) return '—';
    const days = Math.floor(totalSecs / 86400);
    const hours = Math.floor((totalSecs % 86400) / 3600);
    const minutes = Math.floor((totalSecs % 3600) / 60);
    if (days > 0) return `${days}일 ${hours}시간`;
    if (hours > 0) return `${hours}시간 ${minutes}분`;
    return `${minutes}분`;
}

/** Drive letter ("C:") from a disk display name like "C:\ (Windows)". */
export function driveLetter(name) {
    if (!name) return '';
    return name.match(/^[A-Za-z]:/)?.[0] ?? name.slice(0, 2);
}

/**
 * Compare two version strings ("v0.9.4", "0.10.0", "1.0.0-beta.1").
 * Returns 1 if a > b, -1 if a < b, 0 if equal. A pre-release sorts before its release.
 */
export function compareVersions(a, b) {
    const parse = (v) => {
        const [core, pre = ''] = String(v).trim().replace(/^v/i, '').split('-', 2);
        return { nums: core.split('.').map((p) => parseInt(p, 10) || 0), pre };
    };
    const pa = parse(a);
    const pb = parse(b);
    const len = Math.max(pa.nums.length, pb.nums.length);
    for (let i = 0; i < len; i++) {
        const d = (pa.nums[i] ?? 0) - (pb.nums[i] ?? 0);
        if (d !== 0) return d > 0 ? 1 : -1;
    }
    if (pa.pre === pb.pre) return 0;
    if (!pa.pre) return 1;
    if (!pb.pre) return -1;
    return pa.pre > pb.pre ? 1 : -1;
}
