export type JsonDrop = {
    drop_id: number;
    owner_id: string,
    deposit_per_use: string;
    drop_type: JsonDropType;
    config: DropConfig | null;
    metadata: string | null;
    registered_uses: number;
    required_gas: string;
    next_key_id: number;
}

export type JsonDropType = string | JsonNFTData | FTData | FCData;

export type JsonNFTData = {
    sender_id: string;
    contract_id: string;
    longest_token_id: string;
    storage_for_longest: string;
}

export type FTData = {
    sender_id: string;
    contract_id: string;
    balance_per_use: string;
    ft_storage: string;
}

export type FCData = {
    methods: (MethodData | null)[]
    config: FCConfig | null;
}

export type FCConfig = {
    account_id_field: string | null;
    drop_id_field: string | null;
    key_id_field: string | null;
    attached_gas: string | null;
}

export type MethodData = {
    receiver_id: string;
    method_name: string;
    args: string;
    attached_deposit: string;
}

export type JsonKeyInfo = {
    drop_id: number;
    pk: string;
    key_info: KeyInfo
}

export type KeyInfo = {
    remaining_uses: number;
    last_used: number;
    allowance: number;
    key_id: number;
}

export type DropConfig = {
    uses_per_key: number | null;
    start_timestamp: number | null;
    throttle_timestamp: number | null;
    on_claim_refund_deposit: boolean | null;
    claim_permission: string | null;
    drop_root: string | null;
}