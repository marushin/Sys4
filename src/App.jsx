import { useCallback, useEffect, useState } from 'react';
import './App.css';

import ResourcePanel from './components/ResourcePanel';
import TitleBar from './components/TitleBar';
import CheckUpdate from './components/CheckUpdate';
import { loadAppConfig, getConfigValue, setConfigValue } from './utils/config';
import { useSystemResources } from './hooks/useSystemResources';
import { useSavePositionOnMove } from './hooks/useWindowBehavior';

const KEY_EXPANDED = 'sys4_expanded_sections';
const SECTIONS = ['cpu', 'ram', 'gpu', 'network', 'disk'];
// The main window always samples everything so collapsed headers still show live summaries.
const POLL_OPTIONS = { cpu: true, ram: true, gpu: true, network: true, disk: true };

function parseExpanded(raw) {
    let parsed = null;
    try {
        parsed = typeof raw === 'string' ? JSON.parse(raw) : raw;
    } catch {
        // fall through to defaults
    }
    return Object.fromEntries(
        SECTIONS.map((s) => [s, parsed && typeof parsed === 'object' && s in parsed ? Boolean(parsed[s]) : true]),
    );
}

function App() {
    const [expanded, setExpanded] = useState(() => parseExpanded(getConfigValue(KEY_EXPANDED)));
    const [configLoaded, setConfigLoaded] = useState(false);
    const { data: systemResources, error } = useSystemResources(POLL_OPTIONS);

    useSavePositionOnMove('main_window_pos_x', 'main_window_pos_y');

    // Window placement is handled by the backend at start-up; here we only restore UI state.
    useEffect(() => {
        loadAppConfig().then((config) => {
            if (config[KEY_EXPANDED]) setExpanded(parseExpanded(config[KEY_EXPANDED]));
            setConfigLoaded(true);
        });
    }, []);

    useEffect(() => {
        if (configLoaded) setConfigValue(KEY_EXPANDED, JSON.stringify(expanded));
    }, [expanded, configLoaded]);

    const toggleExpanded = useCallback((section) => {
        setExpanded((prev) => ({ ...prev, [section]: !prev[section] }));
    }, []);

    return (
        <div className="app">
            <TitleBar />
            <main className="main-content">
                <ResourcePanel
                    systemResources={systemResources}
                    stale={error}
                    expanded={expanded}
                    onToggleExpanded={toggleExpanded}
                />
            </main>
            <CheckUpdate />
        </div>
    );
}

export default App;
