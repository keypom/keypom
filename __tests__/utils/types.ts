export type ExtDrop = {
    assets_by_use: Record<number, Array<ExtAsset>>;
    internal_assets_data: Array<InternalAsset | null>;
}

export type InternalAsset = InternalFTData;

export type InternalFTData = {
    contract_id: string;
    registration_cost: string,
    balance_avail: string
}

export type ExtAsset = ExtFTData;

export type ExtFTData = {
    contract_id: string;
    registration_cost: string,
    amount: string
}
