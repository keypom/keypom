export type JsonDrop = {
    drop_id: number;
    owner_id: string,
    deposit_per_use: string;
    drop_type: DropType;
    config: DropConfig | null;
    metadata: string | null;
    registered_uses: number;
    required_gas: string;
    next_key_id: number;
}

export type Error = {
    message: String;
}

interface DropType {
    FunctionCall: FCData;
    NonFungibleToken: JsonNFTData;
    FungibleToken: FTData;
}

export type JsonNFTData = {
    sender_id: string;
    contract_id: string;
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
    delete_on_empty: boolean | null;
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