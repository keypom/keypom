export type JsonDrop = {
    drop_id: string;
    owner_id: string,
    deposit_per_use: string;
    simple?: SimpleData;
    nft?: JsonNFTData;
    ft?: FTData;
    fc?: FCData;
    config: DropConfig | null;
    metadata: string | null;
    registered_uses: number;
    required_gas: string;
    next_key_id: number;
}

export type JsonToken = {
    series_id: number;
    token_id: string;
    owner_id: string;
    metadata: TokenMetadata;
    approved_account_ids: Record<string, number>;
    royalty: Record<string, number> | null;
}

export type JsonNFTData = {
    sender_id: string;
    contract_id: string;
}

export type SimpleData = {
    lazy_register?: boolean
}

export type FTData = {
    sender_id: string;
    contract_id: string;
    balance_per_use: string;
}

export type FCData = {
    methods: (MethodData | null)[]
    config?: FCConfig;
}

export type FCConfig = {
    account_id_field?: string;
    drop_id_field?: string;
    key_id_field?: string;
    attached_gas?: string
}

export type MethodData = {
    receiver_id: string;
    method_name: string;
    args: string;
    attached_deposit: string;
}

export type JsonKeyInfo = {
    drop_id: string;
    pk: string;
    // How many uses this key has left. Once 0 is reached, the key is deleted
    remaining_uses: number,
    // When was the last time the key was used
    last_used: number,
    // How much allowance does the key have left. When the key is deleted, this is refunded to the funder's balance.
    allowance: number,
    // Nonce for the current key.
    key_id: number,
    // Which use is the key currently on?
    cur_key_use: number
}

export type KeyInfo = {
    remaining_uses: number;
    last_used: number;
    allowance: number;
    key_id: number;
}

export type TimeConfig = {
    start?: number;
    end?: number;
    throttle?: number;
    interval?: number;
}

export type UsageConfig = {
    permissions?: string;
    refund_deposit?: boolean;
    auto_delete_drop?: boolean;
    auto_withdraw?: boolean;
}

export type DropConfig = {
    uses_per_key?: number;
    time?: TimeConfig;
    usage?: UsageConfig;
    root_account_id?: string;
}

export type TokenMetadata = {
    title: string | null;
    description: string | null;
    media: string | null;
    media_hash: string | null;
    copies: number | null;
    issued_at: number | null;
    expires_at: number | null;
    starts_at: number | null;
    updated_at: number | null;
    extra: string | null;
    reference: string | null;
    reference_hash: string | null;
}