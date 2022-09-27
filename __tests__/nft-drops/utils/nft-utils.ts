import { NearAccount } from "near-workspaces"
import { LARGE_GAS } from "../../utils/general";

export const nftSeriesMetadata = {
    "spec": "nft-1.0.99",
    "name": "NFT Drop Series Contract",
    "symbol": "NCBNFT",
    "base_uri": "https://cloudflare-ipfs.com/ipfs/"
}

export const nftMetadata = {
    "media": "bafybeihnb36l3xvpehkwpszthta4ic6bygjkyckp5cffxvszbcltzyjcwi",
    "title": "This is my title",
    "description": "This is my description",
    "copies": 1000,
}

export const injected_fields = {
    "account_id_field": "receiver_id",
    "drop_id_field": "mint_id"
}

export async function sendNFTs(
    minter: NearAccount,
    tokenIds: String[],
    keypom: NearAccount,
    nftSeries: NearAccount,
    dropIds: String[]
) {
    for(var i = 0; i < tokenIds.length; i++) {
        await minter.call(nftSeries, "nft_transfer_call", {
            receiver_id: keypom,
            token_id: tokenIds[i],
            msg: dropIds[i] 
        },{gas: LARGE_GAS, attachedDeposit: "1"});
    }
}