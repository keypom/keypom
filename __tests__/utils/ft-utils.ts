import { NEAR, NearAccount } from "near-workspaces"
import { LARGE_GAS, functionCall } from "./general";
import { BN } from "bn.js";

export const oneGtNear = new BN("1000000000000000000000000")
export const totalSupply = oneGtNear.mul(new BN('1000000'))
export const ftRegistrationFee = NEAR.parse("0.00125")

export async function sendFTs(
    minter: NearAccount,
    amount: String,
    keypom: NearAccount,
    ftContract: NearAccount,
    dropId: String
) {

    await functionCall({
        signer: minter,
        receiver: ftContract,
        methodName: 'ft_transfer_call',
        args: {
            receiver_id: keypom,
            amount,
            msg: dropId 
        },
        gas: LARGE_GAS,
        attachedDeposit: "1"
    })
}