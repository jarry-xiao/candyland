pub struct Asset {
    pub storage_standard: AssetStorageStandard,
}

pub enum AssetStorageStandard {
    Compressed,
    Decompressed,
}
