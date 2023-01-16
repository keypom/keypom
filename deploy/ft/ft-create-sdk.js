// In this script, we check if the funder owns the NFTs they are adding to the contract. If they don't, throw an error
const { FUNDING_ACCOUNT_ID, FUNDER_INFO, NETWORK_ID, NUM_KEYS, DROP_METADATA, DEPOSIT_PER_USE_NEAR, DROP_CONFIG, KEYPOM_CONTRACT, NFT_DATA, NFT_CONTRACT_ID, NFT_METADATA, FT_DATA, FT_CONTRACT_ID} = require("./configurations");

const path = require("path");
const homedir = require("os").homedir();
const { writeFile, mkdir, readFile } = require('fs/promises');
const { initKeypom, createDrop, getDrops } = require("keypom-js");
const { BN } = require("bn.js");
const { parseNearAmount, formatNearAmount } = require("near-api-js/lib/utils/format");

const { initiateNearConnection } = require("../utils/general");
const { format } = require("path");


// Funder is account to sign txns, can be changed in ./configurations.js
async function createFTDrop(){
    // USER'S RESPONSIBILITY TO CHANGE DEFAULT CONSTS IN CONFIGURATIONS.JS

    // Initialize keypom, this takes care of the new NEAR connection
    console.log("Initiating NEAR connection");
    let near = await initiateNearConnection(NETWORK_ID);
    await initKeypom({near: near, funder: FUNDER_INFO});

    const fundingAccount = await near.account(FUNDING_ACCOUNT_ID);

	// Get amount to transfer and see if owner has enough balance to fund drop
	let amountToTransfer = new BN(FT_DATA.amount).mul(new BN(NUM_KEYS * DROP_CONFIG.usesPerKey)).toString()
	console.log('amountToTransfer: ', amountToTransfer);
    
    let funderFungibleTokenBal = await fundingAccount.viewFunction(
		FT_CONTRACT_ID, 
		'ft_balance_of', 
		{
			account_id: FUNDING_ACCOUNT_ID
		}
	);

	if (new BN(funderFungibleTokenBal).lte(new BN(amountToTransfer))){
		throw new Error('funder does not have enough Fungible Tokens for this drop. Top up and try again.');
	}

    // Creates the FT drop based on data from config file. Keys are automatically generated within the function based on `NUM_KEYS`. Since there is no entropy, all keys are completely random.
    const {keys} = await createDrop({
        numKeys: NUM_KEYS,
        depositPerUseNEAR: DEPOSIT_PER_USE_NEAR,
        metadata: DROP_METADATA,
        config: DROP_CONFIG,
        ftData: FT_DATA,
    });
    pubKeys = keys.publicKeys

    console.log(pubKeys)
    var dropInfo = {};
    // Creating list of pk's and linkdrops; copied from orignal simple-create.js
    for(var i = 0; i < keys.keyPairs.length; i++) {
        let linkdropUrl = NETWORK_ID == "testnet" ? `https://testnet.mynearwallet.com/linkdrop/${KEYPOM_CONTRACT}/${keys.secretKeys[i]}` : `https://mynearwallet.com/linkdrop/${KEYPOM_CONTRACT}/${keys.secretKeys[i]}`;
	    dropInfo[pubKeys[i]] = linkdropUrl;
		console.log(linkdropUrl);
	}
	// Write file of all pk's and their respective linkdrops
	console.log('curPks: ', pubKeys)
	await writeFile(path.resolve(__dirname, `linkdrops.json`), JSON.stringify(dropInfo));
}

createFTDrop();