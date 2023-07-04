#![cfg_attr(target_arch = "wasm32", no_std)]
#![cfg_attr(target_arch = "wasm32", feature(alloc_error_handler))]

/// storage keys used by this contract because it uses raw storage key value writes and reads
const RULES_KEY: &[u8] = b"r";
const FLOOR_KEY: &[u8] = b"f";
/// register constants used
const REGISTER_0: u64 = 0;
/// string literals (improve readability)
const DOUBLE_QUOTE_BYTE: u8 = b'\"';
const RECEIVER_HEADER: &str = "\"|kR|\":";
const ACTION_HEADER: &str = "\"|kA|\":";
const PARAM_STOP: &str = "|kS|\"";
const COMMA: &str = ",";
const ANY_METHOD: &str = "*";
const CALLBACK_GAS: u64 = 20_000_000_000_000;
const YOCTO_PER_GAS_UNIT: Balance = 100_000_000;

/// repeated string literals (in parsing tx payloads)
const DEPOSIT: &str = "|kP|deposit";
const CALLBACK_METHOD_NAME: &str = "callback";

extern crate alloc;

/// DEBUGGING REMOVE
// use alloc::format;


use alloc::vec;
use alloc::vec::Vec;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use core::convert::TryInto;

mod sys;
use sys::*;
mod parse;
use parse::*;

#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[cfg(target_arch = "wasm32")]
#[panic_handler]
#[no_mangle]
pub unsafe fn on_panic(_info: &::core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}

#[cfg(target_arch = "wasm32")]
#[alloc_error_handler]
#[no_mangle]
pub unsafe fn on_alloc_error(_: core::alloc::Layout) -> ! {
    core::arch::wasm32::unreachable()
}

#[no_mangle]
pub fn setup() {
	// check if funder is calling to switch rules, if not funder and rules set, panic
	unsafe {
		if near_sys::storage_has_key(RULES_KEY.len() as u64, RULES_KEY.as_ptr() as u64) == 1 {
			let rules_str = storage_read_str(RULES_KEY);
			let funder = get_string(&rules_str, "|kP|funder").to_string();
			let predecessor_account_id = sys_account_id(1);
			if funder != predecessor_account_id {
				log("only funder");
				sys::panic();
			}
		}
	}
	
    let input_str = get_input(true);
	swrite(RULES_KEY, input_str.as_bytes());
	let floor = account_balance();
    swrite(FLOOR_KEY, &floor.to_le_bytes());
}

#[no_mangle]
pub fn execute() {

	let rules_str = &storage_read_str(RULES_KEY);

	let contracts: Vec<&str> = get_string(rules_str, "|kP|contracts").split(",").collect();
	let methods: Vec<Vec<&str>> = get_string(rules_str, "|kP|methods").split(",").map(|s| s.split(":").collect()).collect();
	let amounts: Vec<Balance> = get_string(rules_str, "|kP|amounts")
		.split(",")
		.map(|a| {
			let amount: Balance = a.parse().ok().unwrap_or_else(|| sys::panic());
			amount
		})
		.collect();

	// args
	let input_str = get_input(true);
    
	// transactions
	let mut transactions: Vec<&str> = input_str.split(RECEIVER_HEADER).collect();
	transactions.remove(0);

	// promise ids for each tx
	let mut promises: Vec<u64> = vec![];

	// execute transactions
	while transactions.len() > 0 {
		let tx = transactions.remove(0);

		let (receiver_id_str, tx_rest) = split_once(tx, COMMA);
		let receiver_id = receiver_id_str[1..receiver_id_str.len()-1].to_string();
		// log(receiver_id);

		let receiver_index_option = contracts.iter().position(|c| c == &receiver_id);
		if receiver_index_option.is_none() {
			sys::panic()
		}
		let receiver_index = receiver_index_option.unwrap();

		let id = if promises.len() == 0 {
			create_promise_batch(receiver_id, None)
		} else {
			create_promise_batch(receiver_id, Some(promises[promises.len() - 1]))
		};
		promises.push(id);

		// actions for tx
		let mut actions: Vec<&str> = tx_rest.split(ACTION_HEADER).collect();
		actions.remove(0);
		let mut action_gas = 0;
		let mut action_deposits: Balance = 0;
		
		while actions.len() > 0 {
			let action = actions.remove(0);

			let (mut action_type, params) = split_once(action, COMMA);
			action_type = &action_type[1..action_type.len()-1];
			// log(action_type);
			// log(params);

			// TODO do we support NEAR transfers?


			match action_type.as_bytes() {
				b"FunctionCall" => {
					let method_name = get_string(params, "|kP|methodName");

					if methods[receiver_index][0] != ANY_METHOD && !methods[receiver_index].contains(&method_name) {
						sys::panic()
					}
					let args = &get_string(params, "|kP|args")
						.replace("\\\\", "\\");
					let deposit = get_u128(params, DEPOSIT);
					// check if deposit exceeds allowed limit for function calls for this contract
					if deposit > amounts[receiver_index] {
						sys::panic()
					}
					action_deposits += deposit;
					let gas = get_u128(params, "|kP|gas") as u64;
					action_gas += gas;
					
					unsafe {
						near_sys::promise_batch_action_function_call(
							id,
							method_name.len() as u64,
							method_name.as_ptr() as u64,
							args.len() as u64,
							args.as_ptr() as u64,
							deposit.to_le_bytes().as_ptr() as u64,
							gas,
						);
					};
				}
				_ => {}
			}
		}

		// after all action promise calls have been added to the batch, promise.then call a new self callback call
		unsafe {
			let cb_id = create_promise_batch(sys_account_id(0), Some(id));
			promises.push(cb_id);
			// all deposits and gas attached to actions count against the floor and used gas up to this call (ignore callback gas)
			let callback_deposit: u64 = 0;
			let args = format!("{},{}", action_deposits, action_gas + near_sys::used_gas());
			near_sys::promise_batch_action_function_call(
				cb_id,
				CALLBACK_METHOD_NAME.len() as u64,
				CALLBACK_METHOD_NAME.as_ptr() as u64,
				args.len() as u64,
				args.as_ptr() as u64,
				callback_deposit.to_le_bytes().as_ptr() as u64,
				CALLBACK_GAS,
			);
		}
	}
}

#[no_mangle]
pub unsafe fn callback() {
	// log(CALLBACK_METHOD_NAME);

	near_sys::promise_result(0, REGISTER_0);
	let result_bytes = register_read(REGISTER_0);
	let result = alloc::str::from_utf8(&result_bytes).ok().unwrap_or_else(|| sys::panic());
	
	if result == "false" {
		return log("promise false");
	}

	// parse the attachedDeposit from the call
    let input_str = get_input(false);
	let (attached_deposit_str, prepaid_gas_str) = split_once(&input_str, ",");
	let attached_deposit: Balance = attached_deposit_str.parse().ok().unwrap_or_else(|| sys::panic());
	let prepaid_gas: Balance = prepaid_gas_str.parse().ok().unwrap_or_else(|| sys::panic());
	let gas_cost = prepaid_gas * YOCTO_PER_GAS_UNIT;

	// update floor
	let mut floor = get_floor();
	floor = floor - attached_deposit - gas_cost;
    swrite(FLOOR_KEY, &floor.to_le_bytes());
}

fn can_exit(rules_str: &str) -> Option<Balance> {
	// rules
	let repay: Balance = get_u128(rules_str, "|kP|repay");
	let floor_exit: Balance = get_u128(rules_str, "|kP|floor");
	let account_balance = account_balance();
	// log(&format!("repay: {}", repay));
	// log(&format!("floor_exit: {}", floor_exit));
	// log(&format!("account_balance: {}", account_balance));
	// repay
	if account_balance < repay {
		log("cannot repay");
		return None;
	}
	// floor
	let floor = get_floor();
	// log(&format!("floor: {}", floor));
	if floor > floor_exit {
		log("floor > floor_exit");
		return None;
	}

	Some(repay)
}

#[no_mangle]
pub fn create_account_and_claim() {
	let rules_str = storage_read_str(RULES_KEY);
	let funder = get_string(&rules_str, "|kP|funder").to_string();
	let predecessor_account_id = sys_account_id(1);

	// allow funder to claim trial with new full access key at any time
	let refund_id = if funder != predecessor_account_id {
		// non-funder e.g. user will have to pass exit conditions
		let exit_option = can_exit(&rules_str);
		let repay = exit_option.unwrap_or_else(|| sys::panic());

		// non-funder will have to repay funder from this account
		let refund_id = create_promise_batch(funder, None);
		unsafe {
			near_sys::promise_batch_action_transfer(
				refund_id,
				repay.to_le_bytes().as_ptr() as u64,
			)
		}
		Some(refund_id)
	} else {
		None
	};
	
	// parse the input and get the public key
    let input_str = get_input(true);	
	let (_, mut public_key_str) = split_once(&input_str, "\"new_public_key\":\"");
	public_key_str = &public_key_str[..public_key_str.len() - 2];
	log(&format!("public_key_str1: {}", public_key_str));
	
	// If the key contains ed25519: prefix, remove it.
	if public_key_str.contains("ed25519:") {
		public_key_str = split_once(public_key_str, "ed25519:").1;
		log(&format!("public_key_str2: {}", public_key_str));
	}

	let public_key = string_to_base58(public_key_str);
	log(&format!("public_key: {:?}", public_key));
	// log(&format!("public_key_len: {}", public_key.len()));

	// cleanup account storage, keys, deploy empty string for contract
	storage_remove(RULES_KEY);
	storage_remove(FLOOR_KEY);
	// promise for add key .then from refund make sure refund finishes first
	let exit_id = create_promise_batch(sys_account_id(0), refund_id);
	unsafe {
		near_sys::promise_batch_action_deploy_contract(
			exit_id,
			0,
			"".as_ptr() as u64,
		);
		let signer_pk_bytes = sys_signer_pk_bytes();

		near_sys::promise_batch_action_delete_key(
			exit_id,
			signer_pk_bytes.len() as u64,
			signer_pk_bytes.as_ptr() as u64,
		);
		near_sys::promise_batch_action_add_key_with_full_access(
			exit_id,
			public_key.len() as u64,
			public_key.as_ptr() as u64,
			0,
		);
	}
}

/// helpers

pub fn get_floor() -> Balance {
	let floor_bytes = storage_read(FLOOR_KEY);
	Balance::from_le_bytes(floor_bytes.try_into().ok().unwrap_or_else(|| sys::panic()))
}

/// views

#[no_mangle]
pub(crate) unsafe fn get_rules() {
	// get rules str pop last char '}'
	let mut rules = storage_read_str(RULES_KEY);
	rules.pop();
	// get current floor value as string
	let floor_str = get_floor().to_string();
	// inject and return bytes
    return_bytes(&format!("{},\"current_floor\":\"{}\"}}", rules, floor_str).as_bytes(), true);
}

#[no_mangle]
pub(crate) unsafe fn get_key_information() {
	let exit_option = can_exit(&storage_read_str(RULES_KEY));
    return_value(format!("{{\"required_gas\":\"50000000000000\",\"yoctoNEAR\": \"0\",\"balance\":\"0\",\"trial_data\":{{\"exit\":{}}}}}", exit_option.is_some()).as_bytes());
}
