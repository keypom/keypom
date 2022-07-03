import React from 'react';
import { createRoot } from 'react-dom/client';
import App from './App';
import { AppProvider } from './state/app.js';
import { BrowserRouter } from "react-router-dom";

const container = document.getElementById('root');
const root = createRoot(container);
root.render(<AppProvider>
	<BrowserRouter>
		<App />
	</BrowserRouter>
</AppProvider>);
