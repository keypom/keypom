const { BN } = require("bn.js");
const { parseNearAmount, formatNearAmount } = require("near-api-js/lib/utils/format");
const { connect, KeyPair, keyStores, utils } = require("near-api-js");
const path = require("path");
const homedir = require("os").homedir();

/// How much Gas each each cross contract call with cost to be converted to a receipt
const GAS_PER_CCC = 5000000000000; // 5 TGas
const RECEIPT_GAS_COST = 2500000000000; // 2.5 TGas
const YOCTO_PER_GAS = 100000000; // 100 million
const ATTACHED_GAS_FROM_WALLET = 100000000000000; // 100 TGas

/// How much yoctoNEAR it costs to store 1 access key
const ACCESS_KEY_STORAGE = new BN("1000000000000000000000");

// Initiate the connection to the NEAR blockchain.
const initiateNearConnection = async (network) => {
	const CREDENTIALS_DIR = ".near-credentials";

	const credentialsPath = (await path).join(homedir, CREDENTIALS_DIR);
	(await path).join;
	let keyStore = new keyStores.UnencryptedFileSystemKeyStore(credentialsPath);

	let nearConfig = {
		networkId: network,
		keyStore,
		nodeUrl: `https://rpc.${network}.near.org`,
		walletUrl: `https://wallet.${network}.near.org`,
		helperUrl: `https://helper.${network}.near.org`,
		explorerUrl: `https://explorer.${network}.near.org`,
	};

	near = await connect(nearConfig);
	return near;
};

// Initiate the connection to the NEAR blockchain.
const estimateRequiredDeposit = async (
    near,
    depositPerUse,
    numKeys,
    usesPerKey,
    attachedGas,
    fcData,
    ftData
) => {
    let totalRequiredStorage = new BN(parseNearAmount("0.2"));
    console.log('totalRequiredStorage: ', totalRequiredStorage.toString())

    let actualAllowance = estimatePessimisticAllowance(attachedGas);
    console.log('actualAllowance: ', actualAllowance.toString())

    let totalAllowance = actualAllowance.mul(new BN(numKeys));
    console.log('totalAllowance: ', totalAllowance.toString())

    let totalAccessKeyStorage = ACCESS_KEY_STORAGE.mul(new BN(numKeys));
    console.log('totalAccessKeyStorage: ', totalAccessKeyStorage.toString())

    let {numNoneFcs, depositRequiredForFcDrops} = getNoneFcsAndDepositRequired(fcData, usesPerKey);
    let totalDeposits = new BN(depositPerUse).mul(new BN(usesPerKey - numNoneFcs)).mul(new BN(numKeys));
    console.log('totalDeposits: ', totalDeposits.toString())

    let totalDepositsForFc = depositRequiredForFcDrops.mul(new BN(numKeys));
    console.log('totalDepositsForFc: ', totalDepositsForFc.toString())

    let requiredDeposit = totalRequiredStorage
        .add(totalAllowance)
        .add(totalAccessKeyStorage)
        .add(totalDeposits)
        .add(totalDepositsForFc);
    
    console.log('requiredDeposit B4 FT costs: ', requiredDeposit.toString())
    
    if (ftData != null) {
        let extraFtCosts = await getFtCosts(near, numKeys, usesPerKey, ftData.contract_id);
        requiredDeposit = requiredDeposit.add(new BN(extraFtCosts));

        console.log('requiredDeposit AFTER FT costs: ', requiredDeposit.toString())
    }

    return requiredDeposit.toString();
};

// Estimate the amount of allowance required for a given attached gas.
const estimatePessimisticAllowance = (attachedGas) => {
    // Get the number of CCCs you can make with the attached GAS
    let numCCCs = Math.floor(attachedGas / GAS_PER_CCC);
    console.log('numCCCs: ', numCCCs)
    // Get the constant used to pessimistically calculate the required allowance
    let powOutcome = Math.pow(1.03, numCCCs);
    console.log('powOutcome: ', powOutcome)

    let requiredGas = (attachedGas + RECEIPT_GAS_COST) * powOutcome + RECEIPT_GAS_COST;
    console.log('requiredGas: ', requiredGas)
    let requiredAllowance = new BN(requiredGas).mul(new BN(YOCTO_PER_GAS));
    console.log('requiredAllowance: ', requiredAllowance.toString())
    return requiredAllowance;
};

// Estimate the amount of allowance required for a given attached gas.
const getNoneFcsAndDepositRequired = (fcData, usesPerKey) => {
    let depositRequiredForFcDrops = new BN(0);
    let numNoneFcs = 0;
    if (fcData == null) {
        return {numNoneFcs, depositRequiredForFcDrops};
    }

    let numMethodData = fcData.methods.length;

    // If there's one method data specified and more than 1 claim per key, that data is to be used
    // For all the claims. In this case, we need to tally all the deposits for each method in all method data.
    if (usesPerKey > 1 && numMethodData == 1) {
        let methodData = fcData.methods[0];

        // Keep track of the total attached deposit across all methods in the method data
        let attachedDeposit = new BN(0);
        for (let i = 0; i < methodData.length; i++) {
            attachedDeposit = attachedDeposit.add(new BN(methodData[i].attachedDeposit));
        }

        depositRequiredForFcDrops = depositRequiredForFcDrops.add(attachedDeposit).mul(usesPerKey);

        return {
            numNoneFcs,
            depositRequiredForFcDrops,
        }
    }
    // In the case where either there's 1 claim per key or the number of FCs is not 1,
    // We can simply loop through and manually get this data
    for (let i = 0; i < numMethodData; i++) {
        let methodData = fcData.methods[i];
        let isNoneFc = methodData == null;
        numNoneFcs += isNoneFc;

        if (!isNoneFc) {
            // Keep track of the total attached deposit across all methods in the method data
            let attachedDeposit = new BN(0);
            for (let j = 0; j < methodData.length; j++) {
                attachedDeposit = attachedDeposit.add(new BN(methodData[j].attachedDeposit));
            }

            depositRequiredForFcDrops = depositRequiredForFcDrops.add(attachedDeposit);
        }
    }

    return {
        numNoneFcs,
        depositRequiredForFcDrops,
    } 
};

// Estimate the amount of allowance required for a given attached gas.
const getFtCosts = async (near, numKeys, usesPerKey, ftContract) => {
    const viewAccount = await near.account("foo");
    const storageBalanceBounds = await viewAccount.viewFunction(ftContract, "storage_balance_bounds", {}); 
    console.log('storageBalanceBounds: ', storageBalanceBounds)
    let costs = new BN(storageBalanceBounds.min).mul(new BN(numKeys)).mul(new BN(usesPerKey)).add(new BN(storageBalanceBounds.min));
    console.log('costs: ', costs.toString());
    return costs.toString();
};

module.exports = {
    initiateNearConnection,
    estimateRequiredDeposit,
    estimatePessimisticAllowance,
    getNoneFcsAndDepositRequired,
    getFtCosts,
    ATTACHED_GAS_FROM_WALLET
};