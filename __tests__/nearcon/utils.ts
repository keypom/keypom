import { NEAR, NearAccount } from "near-workspaces";

export const nftMetadata = {
    "media": "bafybeigmwo5tusydasem6bwwpxxgleztkespzltddwwexknvkg6243cxay",
    "copies": 1000,
};

export const dropConfig = {
    uses_per_key: 3,
    on_claim_refund_deposit: true
}

export const keypomMetadata = {
    media: "https://cloudflare-ipfs.com/ipfs/bafybeiaqz47cjbptevqvap7cvkez4pajvpvmjpvu2gkgu534t3sqknryam",
    id: "nearcon-opening-night"
}

export const nftSeriesMetadata = {
    "spec": "nft-1.0.99",
    "name": "NEARCON Beta Keypom NFTs",
    "symbol": "NCBNFT",
    "base_uri": "https://cloudflare-ipfs.com/ipfs/"
}

export function getNEARConFCData (receiver: NearAccount) {
    return {
        methods: [
            null,
            null,
            [{
                receiver_id: receiver,
                method_name: "nft_mint",
                args: "",
                attached_deposit: NEAR.parse("0.012").toString(),
                account_id_field: "receiver_id",
                drop_id_field: "mint_id"
            }]
        ]
    }
}

export const ticketDistro = {
    "Orderly": [
        100,
        100
    ],
    "Few and Far": [
        100,
        50
    ],
    "Cornerstone": [
        100,
        50
    ],
    "MetaPool": [
        100
    ],
    "Ref": [
        100
    ],
    "Bastion": [
        60
    ],
    "Burrow": [
        60
    ],
    "Trisolaris": [
        30
    ],
    "Pembrock": [
        60
    ],
    "Aurigami": [
        60
    ],
    "NF": [
        100
    ],
    "Proximity": [
        100,
        80
    ]
}