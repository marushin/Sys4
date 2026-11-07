// Theme handling shared by the main window and the widget.
// 'dark' / 'light' are explicit choices; anything else follows the OS setting.

export const KEY_THEME = 'theme';

export function applyTheme(theme) {
    const root = document.documentElement;
    root.classList.toggle('dark-mode', theme === 'dark');
    root.classList.toggle('light-mode', theme === 'light');
}

export function isDarkActive() {
    const root = document.documentElement;
    if (root.classList.contains('dark-mode')) return true;
    if (root.classList.contains('light-mode')) return false;
    return window.matchMedia?.('(prefers-color-scheme: dark)').matches ?? false;
}

/** Re-apply the theme saved in localStorage (shared by all windows of the app). */
export function applyStoredTheme() {
    try {
        applyTheme(localStorage.getItem(KEY_THEME));
    } catch {
        // ignore
    }
}
