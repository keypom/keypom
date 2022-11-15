import { BN, KeyPair, NEAR, NearAccount } from "near-workspaces";
import { JsonDrop, JsonKeyInfo } from "./types";

export const DEFAULT_GAS: string = "30000000000000";
export const LARGE_GAS: string = "300000000000000";
export const WALLET_GAS: string = "100000000000000";
export const DEFAULT_DEPOSIT: string = "1000000000000000000000000";
export const GAS_PRICE: BN = new BN("100000000");
export const DEFAULT_TERRA_IN_NEAR: string = "3000000000000000000000";
export const CONTRACT_METADATA = {
  "version": "1.0.0",
  "link": "https://github.com/mattlockyer/proxy/commit/71a943ea8b7f5a3b7d9e9ac2208940f074f8afba",
}

export async function getDropSupplyForOwner(
  keypom: NearAccount,
  ownerId: string
): Promise<number> {
  const dropSupplyForOwner: number = await keypom.view('get_drop_supply_for_owner', {account_id: ownerId});
  return dropSupplyForOwner;
}

export async function getKeySupplyForDrop(
  keypom: NearAccount,
  dropId: string
): Promise<number> {
  const getKeySupplyForDrop: number = await keypom.view('get_key_supply_for_drop', {drop_id: dropId});
  return getKeySupplyForDrop;
}

export async function getKeyInformation(
  keypom: NearAccount,
  publicKey: string
): Promise<JsonKeyInfo> {
  const keyInformation: JsonKeyInfo = await keypom.view('get_key_information', {key: publicKey});
  return keyInformation;
}

export async function getDropInformation(
  keypom: NearAccount,
  dropId: string
): Promise<JsonDrop> {
  const dropInfo: JsonDrop = await keypom.view('get_drop_information', {drop_id: dropId});
  return dropInfo;
}

export async function generateKeyPairs(
  numKeys: number,
): Promise<{ keys: KeyPair[]; publicKeys: string[] }> {
  // Generate NumKeys public keys
  let kps: KeyPair[] = [];
  let pks: string[] = [];
  for (let i = 0; i < numKeys; i++) {
    let keyPair = await KeyPair.fromRandom('ed25519');
    kps.push(keyPair);
    pks.push(keyPair.getPublicKey().toString());
  }
  return {
    keys: kps,
    publicKeys: pks
  }
}

export function defaultCallOptions(
  gas: string = DEFAULT_GAS,
  attached_deposit: string = DEFAULT_DEPOSIT
) {
  return {
    gas: new BN(gas),
    attachedDeposit: new BN(attached_deposit),
  };
}

export function assertBalanceChange(b1: NEAR, b2: NEAR, expected_change: NEAR, precision: number) {
  console.log('expected change: ', expected_change.toString())

  let numToDivide = new BN(Math.ceil(1 / precision));
  let range = expected_change.abs().div(numToDivide);
  console.log('range addition: ', range.toString())

  let acceptableRange = {
    upper: expected_change.abs().add(range), // 1 + .05 = 1.05
    lower: expected_change.abs().sub(range) // 1 - .05  = .95
  }
  let diff = b2.sub(b1).abs();
  console.log(`diff: ${diff.toString()} range: ${JSON.stringify(acceptableRange)}`)
  return diff.gte(acceptableRange.lower) && diff.lte(acceptableRange.upper)
}

export async function queryAllViewFunctions(
  {
  contract,
  drop_id = null,
  key = null,
  from_index = '0',
  limit = 50,
  account_id = null
  }: 
  {
    contract: NearAccount,
    drop_id?: string | null,
    key?: string | null,
    from_index?: string | null,
    limit?: number | null,
    account_id?: string | null
  }
): Promise<{
  keyBalance: string | null,
  keyInformation: JsonKeyInfo | null,
  dropInformation: JsonDrop | null,
  keySupplyForDrop: number | null,
  keysForDrop: JsonKeyInfo[] | null,
  tokenIdsForDrop: string[] | null,
  dropSupplyForOwner: number | null,
  dropsForOwner: JsonDrop[] | null,
  gasPrice: number,
  rootAccount: string,
  feesCollected: string,
  nextDropId: number,
  keyTotalSupply: number,
  keys: JsonKeyInfo[],
}> {
  let getGasPrice: number = await contract.view('get_gas_price', {});
  let getRootAccount: string = await contract.view('get_root_account', {});
  let getFeesCollected: string = await contract.view('get_fees_collected', {});
  let getNextDropId: number = await contract.view('get_next_drop_id', {});
  let keyTotalSupply: number = await contract.view('get_key_total_supply', {});
  let getKeys: JsonKeyInfo[] = await contract.view('get_keys', {from_index, limit});

  let getKeyBalance: string | null = null;
  let getKeyInformation: JsonKeyInfo | null = null;
  if(key != null) {
    getKeyBalance = await contract.view('get_key_balance', {key});
    getKeyInformation = await contract.view('get_key_information', {key});
  }

  let getDropInformation: JsonDrop | null = null;
  let getKeySupplyForDrop: number | null = null;
  let getKeysForDrop: JsonKeyInfo[] | null = null;
  let tokenIdsForDrop: string[] | null = null;
  if(drop_id != null) {
    getDropInformation = await contract.view('get_drop_information', {drop_id});
    getKeySupplyForDrop = await contract.view('get_key_supply_for_drop', {drop_id});
    getKeysForDrop = await contract.view('get_keys_for_drop', {drop_id, from_index, limit});
    tokenIdsForDrop = await contract.view('get_nft_token_ids_for_drop', {drop_id, from_index, limit});
  }

  let dropSupplyForOwner: number | null = null;
  let dropsForOwner: JsonDrop[] | null = null;
  if(account_id != null) {
    dropSupplyForOwner = await contract.view('get_drop_supply_for_owner', {account_id});
    dropsForOwner = await contract.view('get_drops_for_owner', {account_id, from_index, limit});
  }


  return {
    keyBalance: getKeyBalance,
    keyInformation: getKeyInformation,
    dropInformation: getDropInformation,
    keySupplyForDrop: getKeySupplyForDrop,
    keysForDrop: getKeysForDrop,
    tokenIdsForDrop: tokenIdsForDrop,
    dropSupplyForOwner: dropSupplyForOwner,
    dropsForOwner: dropsForOwner,
    gasPrice: getGasPrice,
    rootAccount: getRootAccount,
    feesCollected: getFeesCollected,
    nextDropId: getNextDropId,
    keyTotalSupply: keyTotalSupply,
    keys: getKeys,
  }
}

export async function createSeries(
  {
  account,
  nftContract,
  metadatas,
  ids
  }:
  {
    account: NearAccount,
    nftContract: NearAccount,
    metadatas: string[],
    ids: string[]
  }
) {
  for(let i = 0; i < metadatas.length; i++) {
    let metadata = metadatas[i];
    let id = ids[i];
    
    await account.call(nftContract, 'create_series', {
      metadata,
      mint_id: id,
    }, {attachedDeposit: DEFAULT_DEPOSIT});
  }
}