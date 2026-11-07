// Applied before first paint (loaded synchronously from <head>) so there is no theme flash.
// Kept as an external file so the Content-Security-Policy can forbid inline scripts.
(function () {
    try {
        var theme = localStorage.getItem('theme');
        if (theme === 'dark') document.documentElement.classList.add('dark-mode');
        else if (theme === 'light') document.documentElement.classList.add('light-mode');
    } catch {
        /* storage unavailable */
    }
})();
