import React from 'react';
import { createRoot } from 'react-dom/client';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import App from './App';
import './styles.css';

createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);

// Windows start hidden (visible: false) to avoid the white webview flash.
// rAF is throttled/never fires while hidden — a timer still runs, so use it
// as the reveal path (Rust also shows the window on page-load-finish).
setTimeout(() => {
  void getCurrentWebviewWindow().show().catch(() => {});
}, 50);
