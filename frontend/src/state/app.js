import { State } from '../utils/state';

import { helloWorld } from './hello';
import { initNear } from './near';

// example
const initialState = {
	app: {
		mounted: false
	}
};

export const { appStore, AppProvider } = State(initialState, 'app');

// example app function
export const onAppMount = (message) => async ({ update, getState, dispatch }) => {
	update('app', { mounted: true });
	update('clicked', false);
	update('data', { mounted: true });
	await update('', { data: { mounted: false } });

    console.log('getState', getState());
    
    // testing undefined, null
	await update('clicked', undefined);
	console.log('getState', getState());
	await update('clicked', null);
	console.log('getState', getState());

	update('foo.bar', { hello: true });
	update('foo.bar', { hello: false, goodbye: true });
	update('foo', { bar: { hello: true, goodbye: false } });
	update('foo.bar.goodbye', true);

	dispatch(initNear());

	await new Promise((resolve) => setTimeout(() => {
		console.log('getState', getState());
		resolve();
	}, 2000));

	dispatch(helloWorld(message));
};
