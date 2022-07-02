import React, { memo } from 'react';

const HelloMessage = memo(({ message }) => {
	console.log('rendered: HelloMessage component');
	return <p>Hello { message }</p>;
});

export default HelloMessage;