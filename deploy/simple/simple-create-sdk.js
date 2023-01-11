const { FUNDING_ACCOUNT_ID, NETWORK_ID, NUM_KEYS, DROP_METADATA, DEPOSIT_PER_USE, DROP_CONFIG, KEYPOM_CONTRACT, FUNDER_INFO } = require("./configurations");

const path = require("path");
const homedir = require("os").homedir();
const { writeFile, mkdir, readFile } = require('fs/promises');
const { initKeypom, createDrop, getDrops } = require("keypom-js");

//funder is account to sign txns, defaults to benjiman.testnet
//numKeys default is 10, deposit defalt is 10, default drop config 1 use per key and delete on empty, metadata empty, funderBalance false
async function createSimpleDrop({funderBalance = false}){
    //USER'S RESPONSIBILITY TO CHANGE DEFAULT CONSTS IN CONFIGURATIONS.JS

    //funder = FUNDING_ACCOUNT_ID, numKeys = NUM_KEYS, depositPerUse = DEPOSIT_PER_USE, dropConfig = DROP_CONFIG, metadata = DROP_METADATA, funderBalance = false
    //init keypom, this takes care of the new NEAR connection
    console.log("Initiating NEAR connection");
    initKeypom({network: NETWORK_ID, funder: FUNDER_INFO});

    //create drop, this generates the keys based on the number of keys passed in and uses funder's keypom balance if funderBalance is true (otherwise will sign a txn with an attached deposit)
    const {keys} = createDrop({
        account: FUNDING_ACCOUNT_ID,
        numKeys: NUM_KEYS,
        depositPerUseNEAR: DEPOSIT_PER_USE,
        metadata: DROP_METADATA,
        config: DROP_CONFIG,
        hasBalance: funderBalance
    });
    pubKeys = keys.publicKeys

    var dropInfo = {};
    //creating list of pk's and linkdrops; copied from orignal simple-create.js
    for(var i = 0; i < keys.keyPairs.length; i++) {
		dropInfo[pubKeys[i]] = `https://testnet.mynearwallet.com/linkdrop/${KEYPOM_CONTRACT}/${keys.secretKeys[i]}`;
		console.log(`https://testnet.mynearwallet.com/linkdrop/${KEYPOM_CONTRACT}/${keys.secretKeys[i]}`);
	}
	//write file of all pk's and their respective linkdrops
	console.log('curPks: ', curPks)
	await writeFile(path.resolve(__dirname, `linkdrops.json`), JSON.stringify(curPks));
}

createSimpleDrop();