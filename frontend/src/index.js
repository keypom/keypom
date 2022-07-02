import React from 'react';
import ReactDOM from 'react-dom';
import App from './App';
import { AppProvider } from './state/app.js';
import { BrowserRouter } from "react-router-dom";

ReactDOM.render(
	<AppProvider>
		<BrowserRouter>
			<App />
		</BrowserRouter>
	</AppProvider>,
	document.getElementById('root')
);
