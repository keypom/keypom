import React from 'react'
import {
	Routes,
	Route,
	Link
} from "react-router-dom";

import './Header.scss';

import { Menu } from 'react-feather';

const Links = ({ update, account }) => {
	const hideMenu = () => update('app.menu', false)
	return <nav>
		<Link onClick={hideMenu} to="/">Home</Link>
		{/* <Link onClick={hideMenu} to="/about">About</Link> */}
		{
			account && <>
				<Link onClick={hideMenu} to="/deploy">Deploy</Link>
				<Link onClick={hideMenu} to="/account">Account</Link>
			</>
		}
	</nav>
}

export const Header = ({ pathname, menu, account, update, SidebarLinks }) => {

	return <header>
		<div>
			<p>
				Drop Zone { pathname.length > 1 && '/ ' + pathname.substring(1) }
			</p>
		</div>
		<div>
			<Menu onClick={() => update('app', { menu: !menu })} />
			<Links {...{ update, account }} />
		</div>
		{menu && window.innerWidth < 768 && <div className="mobile">
			<SidebarLinks {...{ update, account }} />
			<Links {...{ update, account }} />
		</div>}
	</header>
}