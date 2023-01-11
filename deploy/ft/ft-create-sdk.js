//in this script, we check if the funder owns the NFTs they are adding to the contract. If they don't, throw an error
const { FUNDING_ACCOUNT_ID, FUNDER_INFO, NETWORK_ID, NUM_KEYS, DROP_METADATA, DEPOSIT_PER_USE, DROP_CONFIG, KEYPOM_CONTRACT, NFT_DATA, NFT_CONTRACT_ID, NFT_METADATA} = require("./configurations");

const path = require("path");
const homedir = require("os").homedir();
const { writeFile, mkdir, readFile } = require('fs/promises');
const { initKeypom, createDrop, getDrops } = require("keypom-js");

//funder is account to sign txns, defaults to benjiman.testnet
//numKeys default is 10, deposit defalt is 10, default drop config 1 use per key and delete on empty, metadata empty, funderBalance false
async function createNFTDropOwned({funderBalance = false}){
    //USER'S RESPONSIBILITY TO CHANGE DEFAULT CONSTS IN CONFIGURATIONS.JS

    //init keypom, this takes care of the new NEAR connection
    console.log("Initiating NEAR connection");
    initKeypom({network: NETWORK_ID, funder: FUNDER_INFO});

	//get amount to transfer and see if owner has enough balance to fund drop
	let amountToTransfer = new BN(FT_DATA.balancePerUse).mul(new BN(NUM_KEYS * DROP_CONFIG.usesPerKey))
	let amountToTransfer_String = amountToTransfer.toString();
	console.log('amountToTransfer: ', amountToTransfer_String);	
	if (await FT_CONTRACT_ID.ft_balance_of({ account_id: FUNDING_ACCOUNT_ID }) < amountToTransfer){
		throw new Error('funder does not have enough FT for this drop');
	}

    //create drop, this generates the keys based on the number of keys passed in and uses funder's keypom balance if funderBalance is true (otherwise will sign a txn with an attached deposit)
    const {keys} = createDrop({
        account: FUNDING_ACCOUNT_ID,
        numKeys: NUM_KEYS,
        depositPerUseNEAR: DEPOSIT_PER_USE,
        metadata: DROP_METADATA,
        config: DROP_CONFIG,
        ftData: FT_DATA,
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

createNFTDropOwned();