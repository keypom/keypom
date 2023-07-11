export type ExtDrop = {
    assets_by_use: Record<number, Array<ExtAsset>>;
    internal_assets_data: Array<InternalAsset | null>;
}
export type ExtKeyInfo = {
    /// How much Gas should be attached when the key is used to call `claim` or `create_account_and_claim`.
    /// It is up to the smart contract developer to calculate the required gas (which can be done either automatically on the contract or on the client-side).
    required_gas: string,

    /// yoctoNEAR$ amount that will be sent to the account that claims the linkdrop (either new or existing)
    /// when the key is successfully used.
    yoctonear: string,

    /// If using the FT standard extension, a set of FTData can be linked to the public key
    /// indicating that all those assets will be sent to the account that claims the linkdrop (either new or
    /// existing) when the key is successfully used.
    ft_list: Array<ExtFTData>, 

    /* CUSTOM */
    uses_remaining: Number
}

export type InternalAsset = InternalFTData | InternalNFTData | "near";

export type InternalFTData = {
    ft: {
        contract_id: string;
        registration_cost: string,
        balance_avail: string
    }
}

export type InternalNFTData = {
    nft: {
        contract_id: string;
        token_ids: Array<string>
    }
}

export type ExtAsset = ExtFTData;

export type ExtFTData = {
    ft_contract_id: string;
    registration_cost: string,
    ft_amount: string
}

export type ExtNearData = {
    yoctonear: string
}
