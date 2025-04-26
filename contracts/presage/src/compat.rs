//! Compatibility layer for different versions of cosmwasm-std types
//! This handles conversion between different versions of the same types

/// Convert from old Uint128 (1.5.11) to new Uint128 (2.2.2)
pub fn uint128_to_new(old_uint: impl Into<u128>) -> cosmwasm_std::Uint128 {
    let value: u128 = old_uint.into();
    cosmwasm_std::Uint128::from(value)
}

/// Convert from new Uint128 (2.2.2) to old Uint128 (1.5.11)
/// This requires you to explicitly import the pyth_sdk_cw's version
pub fn uint128_to_old<T: From<u128>>(new_uint: cosmwasm_std::Uint128) -> T {
    T::from(new_uint.u128())
}

/// Convert from old Addr (1.5.11) to new Addr (2.2.2)
pub fn addr_to_new(old_addr: impl AsRef<str>) -> cosmwasm_std::Addr {
    cosmwasm_std::Addr::unchecked(old_addr.as_ref())
}

/// Convert from new Addr (2.2.2) to string
pub fn addr_to_string(addr: &cosmwasm_std::Addr) -> String {
    addr.to_string()
}