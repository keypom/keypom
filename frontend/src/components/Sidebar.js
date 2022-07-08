import React from 'react'

import {
	Link
} from "react-router-dom";

import './Sidebar.scss'

export const SidebarLinks = ({ pathname, update, account }) => {
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

export const Sidebar = ({ pathname, update, account }) => {
	return <div className="sidebar">
		<SidebarLinks {...{ pathname, update, account }} />
	</div>
}