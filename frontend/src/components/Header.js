import React from 'react'
import {
	Routes,
	Route,
	Link
} from "react-router-dom";

import { Menu } from 'react-feather';


const Links = () => {
	return <nav>
		<Link to="/">Home</Link>
		<Link to="/wallet">Wallet</Link>
	</nav>
}

export const Header = ({ menu, update }) => {
	return <header>
		<div>
			<p>
			Drop Zone
			</p>
		</div>
		<div>
			<Menu onClick={() => update('app', { menu: !menu })} />
			<Links />
		</div>
		{ menu && window.innerWidth < 768 && <Links /> }
	</header>
}