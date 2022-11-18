import { KeyPair, NEAR, NearAccount } from "near-workspaces";
import { generateKeyPairs, LARGE_GAS } from "../../utils/general";
import { DropConfig } from "../../utils/types";

export const nftSeriesMetadata = {
    "spec": "nft-1.0.99",
    "name": "NEARCON Beta Keypom NFTs",
    "symbol": "NCBNFT",
    "base_uri": "https://cloudflare-ipfs.com/ipfs/"
}

export async function createDistro(
    ticketDistribution: Record<string, number[]>, 
    owner: NearAccount, 
    keypom: NearAccount, 
    nftSeries: NearAccount, 
    ownerBalance: string,
    depositPerUse: string,
    ) {
    await owner.updateAccount({
        amount: NEAR.parse('10000 N').toString()
    })
    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse(ownerBalance).toString()});

    let keyPairsForSponsors: Record<string, KeyPair[]> = {};
    let mint_id = 0;
    // Loop through each ticket in the distro and create the drop
    for (let [sponsor, tickets] of Object.entries(ticketDistribution)) {
        console.log(`Creating Series for ${sponsor}`);
        await nftSeries.call(nftSeries, 'create_series', {mint_id, metadata: pagodaNftMetadataPizza}, {attachedDeposit: NEAR.parse("0.02").toString()});

        // Creating the empty drop
        await owner.call(keypom, 'create_drop', {
            deposit_per_use: depositPerUse,
            fc: getNEARConFCData(nftSeries),
            config: null
        },{gas: LARGE_GAS});

        //Creating the tickets for the sponsor
        let finalKeys: KeyPair[] = [];
        for (let i = 0; i < tickets.length; i++) {
            console.log(`Creating ${tickets[i]} tix for ${sponsor}`);
            let {keys, publicKeys} = await generateKeyPairs(tickets[i]);
            // Add the keys vector to the final keys array
            finalKeys.push(...keys);

            await owner.call(keypom, 'add_keys', {
                public_keys: publicKeys, 
                drop_id: mint_id.toString()
            },{gas: LARGE_GAS});
        }

        console.log(`Finished. Incrementing Mint ID. Was ${mint_id} now ${mint_id + 1}`);
        keyPairsForSponsors[sponsor] = finalKeys;
        mint_id += 1;
    }

    let keypomBalance = await keypom.balance();
    console.log('keypom available after distro: ', keypomBalance.available.toString())
    console.log('keypom staked after distro: ', keypomBalance.staked.toString())
    console.log('keypom stateStaked after distro: ', keypomBalance.stateStaked.toString())
    console.log('keypom total after distro: ', keypomBalance.total.toString())

    let nftBalance = await nftSeries.balance();
    console.log('nftSeries available after distro: ', nftBalance.available.toString())
    console.log('nftSeries staked after distro: ', nftBalance.staked.toString())
    console.log('nftSeries stateStaked after distro: ', nftBalance.stateStaked.toString())
    console.log('nftSeries total after distro: ', nftBalance.total.toString())

    return keyPairsForSponsors;
}

export const pagodaNftMetadataBucketHat = {
    "media": "bafybeihnb36l3xvpehkwpszthta4ic6bygjkyckp5cffxvszbcltzyjcwi",
    "title": "This is my bucket hat title",
    "description": "Thank you for supporting our Bucket Hat! Welcome to the NEAR ecosystem.",
    "copies": 1000,
};

export const pagodaNftMetadataPizza = {
    "media": "bafybeihnb36l3xvpehkwpszthta4ic6bygjkyckp5cffxvszbcltzyjcwi",
    "title": "This is my pizza poap title",
    "description": "Thank you for supporting our Pizza Poap! Welcome to the NEAR ecosystem.",
    "copies": 1000,
};

export const dropConfig: DropConfig = {
    uses_per_key: 3,
    usage: {
        refund_deposit: true
    }
}

export const keypomMetadata = {
    media: "https://cloudflare-ipfs.com/ipfs/bafybeiaqz47cjbptevqvap7cvkez4pajvpvmjpvu2gkgu534t3sqknryam",
    id: "nearcon-opening-night"
}

export function getNEARConFCData (receiver: NearAccount) {
    return {
        methods: [
            [{
                receiver_id: receiver,
                method_name: "nft_mint",
                args: "",
                attached_deposit: NEAR.parse("0.015").toString(),
                account_id_field: "receiver_id",
                drop_id_field: "mint_id"
            }]
        ]
    } 
}