import React, { useContext, useEffect } from 'react';
import {
	Routes,
	Route,
	useLocation,
} from "react-router-dom";

import { appStore, onAppMount } from './state/app';
import { Header } from './components/Header';
import { Home } from './components/Home';

import './css/normalize.css';
import './css/skeleton.css';
import './App.scss';

const App = () => {
	const { state, dispatch, update } = useContext(appStore);

	const { app, wallet, account } = state
	const { menu } = app
	const { pathname } = useLocation();

	const onMount = () => {
		dispatch(onAppMount());
	};
	useEffect(onMount, []);

	const routeArgs = {
		state, update
	}

	return (
		<div>
			<Header {...{ pathname, menu, account, update }} />
			{
				account ?
					/* Account Paths */
					<main>
						<Routes>
							<Route path="/account" element={
								<>
									<p>{account.accountId}</p>
									<button onClick={() => wallet.signOut()}>Sign Out</button>
								</>
							} />
							<Route path="/" element={<Home {...routeArgs} />} />
						</Routes>
					</main>
					:
					/* Public Paths */
					<main>
						<Routes>
							<Route path="/about" element={
								<>
									<p>Drop Zone is dope</p>
								</>
							} />
							<Route path="/" element={
								<>
									<p>Please sign in to get started</p>
									<button onClick={() => wallet.signIn()}>Sign In</button>
								</>
							} />
						</Routes>
					</main>
			}

		</div>
	);
};

export default App;
