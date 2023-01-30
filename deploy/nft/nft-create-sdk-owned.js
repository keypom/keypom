// In this script, we check if the funder owns the NFTs they are adding to the contract. If they don't, throw an error
const { FUNDING_ACCOUNT_ID, FUNDER_INFO, NETWORK_ID, NUM_KEYS, DROP_METADATA, DEPOSIT_PER_USE_NEAR, DROP_CONFIG, KEYPOM_CONTRACT, NFT_DATA, NFT_CONTRACT_ID, NFT_METADATA} = require("./configurations");

// NOTE: This script MUST be run on testnet and WILL NOT WORK ON MAINNET
// This is beause the chosen NFT contract for this tutorial lives on testnet.

const path = require("path");
const homedir = require("os").homedir();
const { writeFile, mkdir, readFile } = require('fs/promises');
const { initKeypom, createDrop, getDrops } = require("keypom-js");
const { initiateNearConnection } = require("../utils/general");


// Funder is account to sign txns, can be changed in ./configurations.js
async function createNFTDropOwned(){
    // USER'S RESPONSIBILITY TO CHANGE DEFAULT CONSTS IN CONFIGURATIONS.JS

    // Init keypom, this takes care of the new NEAR connection
    console.log("Initiating NEAR connection");
    let near = await initiateNearConnection(NETWORK_ID);
    await initKeypom({near: near, funder: FUNDER_INFO});

    const fundingAccount = await near.account(FUNDING_ACCOUNT_ID);

    // Get array of token_ids owned by funder
    let funderOwnedNFTs = await fundingAccount.viewFunction(
        NFT_CONTRACT_ID, 
        'nft_tokens_for_owner', 
        {
            account_id: FUNDING_ACCOUNT_ID,
        },
    );
    let funderOwnedTokenIds = [];
    for(i = 0; i<funderOwnedNFTs.length; i++){
        funderOwnedTokenIds.push(funderOwnedNFTs[i].token_id)
    }
    


    // See if funder actually owns the NFTs in NFT_DATA_OWNED
    for(var i = 0; i < NFT_DATA.tokenIds.length; i++){
        if (!funderOwnedTokenIds.includes(NFT_DATA.tokenIds[i])){
            throw new Error(`funder does not own Non Fungible Token with ID: ${NFT_DATA.tokenIds[i]}`);
        }
    }


    // Creates the FT drop based on data from config file. Keys are automatically generated within the function based on `NUM_KEYS`. Since there is no entropy, all keys are completely random.
    const {keys} = await createDrop({
        numKeys: NUM_KEYS,
        depositPerUseNEAR: DEPOSIT_PER_USE_NEAR,
        metadata: DROP_METADATA,
        config: DROP_CONFIG,
        nftData: NFT_DATA,
    });
    pubKeys = keys.publicKeys

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

createNFTDropOwned();