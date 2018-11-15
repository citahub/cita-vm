pub struct GasTable {
    pub ext_code_size: u64,
    pub ext_code_copy: u64,
    pub ext_code_hash: u64,
    pub balance: u64,
    pub sload: u64,
    pub calls: u64,
    pub suicide: u64,
    pub exp_byte: u64,
    pub create_by_suicide: u64,
}

pub const GAS_TABLE: GasTable = GasTable {
    ext_code_size: 700,
    ext_code_copy: 700,
    ext_code_hash: 400,
    balance: 400,
    sload: 200,
    calls: 700,
    suicide: 5000,
    exp_byte: 50,
    create_by_suicide: 25000,
};
