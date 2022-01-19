///! 128 and 256 bit numbers

use uint::construct_uint;

construct_uint! {
    pub struct U128(2);
}

construct_uint! {
    pub struct U256(4);
}
