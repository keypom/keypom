import React, { useContext, useEffect } from 'react';
import {
	Routes,
	Route,
	Link
} from "react-router-dom";

import { appStore, onAppMount } from './state/app';

import HelloMessage from './HelloMessage';

import './App.scss';

const App = () => {
	const { state, dispatch, update } = useContext(appStore);

	console.log('state', state);

	const { wallet, account } = state

	const onMount = () => {
		dispatch(onAppMount('world'));
	};
	useEffect(onMount, []);

	const handleClick = () => {
		update('clicked', !state.clicked);
	};

	return (
		<div>

			<nav>
				<ul>
					<li>
						<Link to="/">Home</Link>
					</li>
					<li>
						<Link to="/hello">Hello</Link>
					</li>
					<li>
						<Link to="/wallet">Wallet</Link>
					</li>
				</ul>
			</nav>

			<Routes>
				<Route path="/wallet" element={
					account ? <>
						<p>{ account.accountId }</p>
						<button onClick={() => wallet.signOut()}>Sign Out</button>
					</> :
					<>
						<p>Not Signed In</p>
						<button onClick={() => wallet.signIn()}>Sign In</button>
					</>
				} />
				<Route path="/hello" element={
					<HelloMessage message={state.foo && state.foo.bar.hello} />
				} />
				<Route path="/" element={
					<>
						<p>clicked: {JSON.stringify(state.clicked)}</p>
						<button onClick={handleClick}>Click Me</button>
					</>
				} />
			</Routes>

		</div>
	);
};

export default App;
