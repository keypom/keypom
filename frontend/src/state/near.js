import * as nearAPI from 'near-api-js';
const { WalletAccount } = nearAPI
import { near } from '../../utils/near-utils';
import getConfig from '../../utils/config';
const { contractId } = getConfig();

export const initNear = () => async ({ update }) => {

	const wallet = new WalletAccount(near)

	wallet.signIn = () => {
		wallet.requestSignIn(contractId, 'Blah Blah');
	};
	const signOut = wallet.signOut;
	wallet.signOut = () => {
		signOut.call(wallet);
		update('', { account: null });
	};

	wallet.signedIn = wallet.isSignedIn();
    
	let account;
	if (wallet.signedIn) {
		account = wallet.account();
	}

	await update('', { near, wallet, account });

};