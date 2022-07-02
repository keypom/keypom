export const helloWorld = (message) => async ({ update }) => {
	update('foo.bar.hello', message);
};