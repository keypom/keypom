use crate::*;

/// helper to get next value from string key in stringified json
pub(crate) fn get_string<'a>(string: &'a str, key: &str) -> &'a str {
    let (_, value) = split_once(string, key);
    let (value, _) = split_once(value, PARAM_STOP);
    &value[3..]
}

/// helper to get and parse the next Balance value from a string key in stringified json
pub(crate) fn get_u128(str: &str, key: &str) -> Balance {
    let amount = get_string(str, key);
    // TODO: This should be minimal, but can explore removing ToStr usage for code size
    amount.parse().ok().unwrap_or_else(|| sys::panic())
}

pub(crate) fn get_input(strip_slashes: bool) -> String {
    unsafe { near_sys::input(REGISTER_0) };
    let input = register_read(REGISTER_0);
    // if from_borsh {
    //     input = input[1..input.len()-1].to_vec();
    // }
	let input_str = alloc::str::from_utf8(&input).ok().unwrap_or_else(|| sys::panic());
    if strip_slashes {
        return input_str.replace("\\\"", "\"");
    }
    input_str.to_string()
}

pub(crate) fn split_once<'a>(string: &'a str, del: &str) -> (&'a str, &'a str) {
    string.split_once(del).unwrap_or_else(|| sys::panic())
}

// decode base58 public keys

const B58_DIGITS_MAP: &'static [i8] = &[
	-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,
	-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,
	-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,
	-1, 0, 1, 2, 3, 4, 5, 6, 7, 8,-1,-1,-1,-1,-1,-1,
	-1, 9,10,11,12,13,14,15,16,-1,17,18,19,20,21,-1,
	22,23,24,25,26,27,28,29,30,31,32,-1,-1,-1,-1,-1,
	-1,33,34,35,36,37,38,39,40,41,42,43,-1,44,45,46,
	47,48,49,50,51,52,53,54,55,56,57,-1,-1,-1,-1,-1,
];

pub(crate) fn string_to_base58(string: &str) -> Vec<u8> {
    let mut bin = [0u8; 132];
    let mut out = [0u32; (132 + 3) / 4];
    let bytesleft = (bin.len() % 4) as u8;
    let zeromask = match bytesleft {
        0 => 0u32,
        _ => 0xffffffff << (bytesleft * 8),
    };

    let zcount = string.chars().take_while(|x| *x == '1').count();
    let mut i = zcount;
    let b58: Vec<u8> = string.bytes().collect();

    while i < string.len() {
        if (b58[i] & 0x80) != 0 {
            // High-bit set on invalid digit
            log("High-bit set on invalid digit");
            sys::panic()
        }

        if B58_DIGITS_MAP[b58[i] as usize] == -1 {
            log("Invalid base58 digit");
            sys::panic()
        }

        let mut c = B58_DIGITS_MAP[b58[i] as usize] as u64;
        let mut j = out.len();
        while j != 0 {
            j -= 1;
            let t = out[j] as u64 * 58 + c;
            c = (t & 0x3f00000000) >> 32;
            out[j] = (t & 0xffffffff) as u32;
        }

        if c != 0 {
            log("Output number too big");
            sys::panic()
        }

        if (out[0] & zeromask) != 0 {
            log("Output number too big");
            sys::panic()
        }

        i += 1;
    }

    let mut i = 1;
    let mut j = 0;

    bin[0] = match bytesleft {
        3 => ((out[0] & 0xff0000) >> 16) as u8,
        2 => ((out[0] & 0xff00) >> 8) as u8,
        1 => {
            j = 1;
            (out[0] & 0xff) as u8
        },
        _ => {
            i = 0;
            bin[0]
        }
    };

    while j < out.len() {
        bin[i] = ((out[j] >> 0x18) & 0xff) as u8;
        bin[i + 1] = ((out[j] >> 0x10) & 0xff) as u8;
        bin[i + 2] = ((out[j] >> 8) & 0xff) as u8;
        bin[i + 3] = ((out[j] >> 0) & 0xff) as u8;
        i += 4;
        j += 1;
    }

    let leading_zeros = bin.iter().take_while(|x| **x == 0).count();
    bin[leading_zeros - zcount - 1..].to_vec()
}

// pub(crate) fn update_string(string: &str, key: &str, val: &str) -> String {
//     let mut ret: String = String::new();
//     let (left, right) = string.split_once(key).unwrap_or_else(|| sys::panic());
//     let (_, right) = right.split_once(PARAM_STOP).unwrap_or_else(|| sys::panic());
    
//     ret.push_str(left);
//     ret.push_str(key);
//     ret.push_str("\",\"");
//     ret.push_str(val);
//     ret.push_str(PARAM_STOP);
//     ret.push_str(right);

//     ret
// }


// const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
// const fn val(c: u8) -> u8 {
//     match c {
//         b'A'..=b'F' => c - b'A' + 10,
//         b'a'..=b'f' => c - b'a' + 10,
//         b'0'..=b'9' => c - b'0',
//         _ => 0
//     }
// }

// const fn byte2hex(byte: u8) -> (u8, u8) {
//     let high = HEX_CHARS[((byte & 0xf0) >> 4) as usize];
//     let low = HEX_CHARS[(byte & 0x0f) as usize];
//     (high, low)
// }

// pub(crate) fn bytes2hex(bytes: &[u8]) -> Vec<u8> {
//     let mut ret = vec![];
// 	for byte in bytes {
// 		if *byte == 0 {
// 			continue;
// 		}
// 		let (byte1, byte2) = byte2hex(*byte);
// 		ret.push(byte1);
// 		ret.push(byte2);
// 	}
//     ret
// }

// pub(crate) fn hex2bytes(hex: &[u8], len: usize) -> Vec<u8> {
//     let mut ret = vec![];
//     for i in (0..len).step_by(2) {
//         ret.push(val(hex[i]) << 4 | val(hex[i+1]))
//     }
//     ret
// }