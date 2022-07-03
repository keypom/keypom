import React, { useContext, useEffect } from 'react';
import {
	Routes,
	Route,
	Link
} from "react-router-dom";

import { appStore, onAppMount } from './state/app';
import { Header } from './components/Header';

import './css/normalize.css';
import './css/skeleton.css';
import './App.scss';

const App = () => {
	const { state, dispatch, update } = useContext(appStore);

	const { wallet, account } = state

	const onMount = () => {
		dispatch(onAppMount());
	};
	useEffect(onMount, []);

	const handleClick = () => {
		update('clicked', !state.clicked);
	};

	return (
		<div>
			<Header {...{ menu: state.app.menu, update }} />
			<main>
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
					<Route path="/" element={
						<>
							<img src="https://musicart.xboxlive.com/7/660e1100-0000-0000-0000-000000000002/504/image.jpg?w=1920&h=1080" />
							<p>clicked: {JSON.stringify(state.clicked)}</p>
							<button onClick={handleClick}>Click Me</button>
						</>
					} />
				</Routes>
			</main>
		</div>
	);
};

export default App;
