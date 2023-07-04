import { NEAR, NearAccount } from "near-workspaces"
import { LARGE_GAS } from "../../utils/general";

export const oneGtNear = BigInt("1000000000000000000000000")
export const totalSupply = oneGtNear * BigInt(1_000_000)
export const ftRegistrationFee = NEAR.parse("0.00125")

export async function sendFTs(
    minter: NearAccount,
    amount: String,
    keypom: NearAccount,
    ftContract: NearAccount,
    dropId: String
) {
    await minter.callRaw(ftContract, "ft_transfer_call", {
        receiver_id: keypom,
        amount,
        msg: dropId 
    },{gas: LARGE_GAS, attachedDeposit: "1"});
}