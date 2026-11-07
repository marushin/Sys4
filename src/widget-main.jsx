import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import Widget from './components/Widget';
import './widget.css';

createRoot(document.getElementById('widget-root')).render(
    <StrictMode>
        <Widget />
    </StrictMode>,
);
