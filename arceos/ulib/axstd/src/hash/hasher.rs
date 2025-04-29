use arceos_api::modules::axhal::misc::random;
use core::hash::{BuildHasher, SipHasher13};

pub struct WrapHasher {
    pub k0: u64,
    pub k1: u64,
}

#[allow(deprecated)]
impl WrapHasher {
    #[allow(deprecated)]
    pub fn new() -> WrapHasher
 {
        WrapHasher
     {
            k0: random() as u64,
            k1: random() as u64,
        }
    }
}


impl BuildHasher for WrapHasher {
    type Hasher = SipHasher13;

    #[allow(deprecated)]
    fn build_hasher(&self) -> SipHasher13 {
        SipHasher13::new_with_keys(self.k0, self.k1)
    }
}