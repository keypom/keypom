export type ExtAssetDataForUses = {
    /// How many uses does this asset data apply to?
    uses: number,
    /// Which assets should be present for these uses
    assets: Array<ExtAsset | undefined>,
    /// Any configurations for this set of uses
    config: UseConfig | undefined
}

export type ExtDrop = {
    /// ID for this specific drop
    drop_id: string,
    /// Account ID who funded / owns the rights to this specific drop
    funder_id: string,
    /// What is the maximum number of uses a given key can have in the drop?
    max_key_uses: number,

    asset_data: Array<ExtAssetDataForUses>,

    nft_asset_data: Array<InternalNFTData>,
    ft_asset_data: Array<InternalFTData>,

    /// Keep track of different configuration options for all the uses of a key in a given drop
    drop_config: DropConfig | undefined,

    /// Keep track of the next nonce to give out to a key
    next_key_id: number
}

export type UserProvidedFCArgs = Array<AssetSpecificFCArgs>;
export type AssetSpecificFCArgs = Array<string | undefined> | undefined;

export type PickOnly<T, K extends keyof T> =
    Pick<T, K> & { [P in Exclude<keyof T, K>]?: never };
    
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
    contract_id: string;
    registration_cost: string,
    balance_avail: string
}

export type InternalNFTData = {
    contract_id: string;
    token_ids: Array<string>
}

export type TokenMetadata = {
    title: string|undefined,
    description: string,
    media: string,
    media_hash: string|undefined,
    copies: number|undefined,
    issued_at: number|undefined,
    expires_at: number|undefined,
    starts_at: number|undefined,
    updated_at: number|undefined,
    extra: string|undefined,
    reference: string|undefined,
    reference_hash: number[]|undefined
}

export type ExtAsset = ExtFTData | ExtNFTData | ExtNearData | MethodData[] | null;

export type ExtFTData = {
    ft_contract_id: string;
    registration_cost: string,
    ft_amount: string
}

export type ExtNFTData = {
    nft_contract_id: string
}

export type ExtNearData = {
    yoctonear: string
}

export type MethodData = {
    /// Contract that will be called
    receiver_id: string,
    /// Method to call on receiver_id contract
    method_name: string,
    /// Arguments to pass in (stringified JSON)
    args: string,
    /// Amount of yoctoNEAR to attach along with the call
    attached_deposit: string,
    /// How much gas to attach to this method call.
    attached_gas: string,

    /// Keypom Args struct to be sent to external contracts
    keypom_args: KeypomInjectedArgs | undefined,
    /// If set to true, the claiming account ID will be the receiver ID of the method call.
    /// Ths receiver must be a valid account and non-malicious (cannot be set to the keypom contract) 
    receiver_to_claimer: boolean | undefined,
    /// What permissions does the user have when providing custom arguments to the function call?
    /// By default, the user cannot provide any custom arguments
    user_args_rule: string | undefined,
}

export type KeypomInjectedArgs = {
    /// Specifies what field the claiming account ID should go in when calling the function
    /// If None, this isn't attached to the args
    account_id_field: string | undefined,
    /// Specifies what field the drop ID should go in when calling the function. To insert into nested objects, use periods to separate. For example, to insert into args.metadata.field, you would specify "metadata.field"
    /// If Some(String), attach drop ID to args. Else, don't attach.
    drop_id_field: string | undefined,
    /// Specifies what field the key ID should go in when calling the function. To insert into nested objects, use periods to separate. For example, to insert into args.metadata.field, you would specify "metadata.field"
    /// If Some(String), attach key ID to args. Else, don't attach.
    key_id_field: string | undefined,
    // Specifies what field the funder id should go in when calling the function. To insert into nested objects, use periods to separate. For example, to insert into args.metadata.field, you would specify "metadata.field"
    // If Some(string), attach the funder ID to the args. Else, don't attach.
    funder_id_field: string | undefined,
}

export type UseConfig = {
    /// Configurations related to how often keys can be used
    time: TimeConfig | undefined,
    
    /// Can the access key for this use call the claim method_name? Default to both method_name callable
    permissions: string | undefined,
    
    /// When calling `create_account` on the root account, which keypom args should be attached to the payload.
    account_creation_keypom_args: KeypomInjectedArgs | undefined,

    /// Override the global root account that sub-accounts will have (near or testnet). This allows
    /// users to create specific drops that can create sub-accounts of a predefined root.
    /// For example, Fayyr could specify a root of `fayyr.near` By which all sub-accounts will then
    /// be `ACCOUNT.fayyr.near`
    root_account_id: string | undefined,
}

export type TimeConfig = {
    /// Minimum block timestamp before keys can be used. If None, keys can be used immediately
    /// Measured in number of non-leap-nanoseconds since January 1, 1970 0:00:00 UTC.
    start: number | undefined,

    /// Block timestamp that keys must be before. If None, keys can be used indefinitely
    /// Measured in number of non-leap-nanoseconds since January 1, 1970 0:00:00 UTC.
    end: number | undefined,

    /// Time interval between each key use. If None, there is no delay between key uses.
    /// Measured in number of non-leap-nanoseconds since January 1, 1970 0:00:00 UTC.
    throttle: number | undefined,

    /// Interval of time after the `start_timestamp` that must pass before a key can be used.
    /// If multiple intervals pass, the key can be used multiple times. This has nothing to do
    /// With the throttle timestamp. It only pertains to the start timestamp and the current
    /// timestamp. The last_used timestamp is not taken into account.
    /// Measured in number of non-leap-nanoseconds since January 1, 1970 0:00:00 UTC.
    interval: number | undefined,
}

export type DropConfig = {
    /// Should the drop be automatically deleted when all the keys are used? This is defaulted to true and
    /// Must be overwritten
    delete_empty_drop: boolean | undefined,

    /// How much extra allowance should be given to each key in the drop?
    /// This allows keys to be used for extra functionalities such as `nft_transfer`, `nft_approve`, etc.
    extra_allowance_per_key: number | undefined
}