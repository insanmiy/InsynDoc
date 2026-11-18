import { createRoot } from 'react-dom/client';
import { App } from './App';
import { desktop } from './bridge';
import { prefetchStartupDocument } from './document-session';
import '@docx-editor.dev/core/styles/editor.css';
import './styles.css';

prefetchStartupDocument(desktop);

createRoot(document.getElementById('root')!).render(<App />);
