pub mod registry;

pub mod generated {
    pub mod omv {
        pub mod contract {
            pub mod v1 {
                include!(concat!(env!("OUT_DIR"), "/omv.contract.v1.rs"));
            }
        }
    }
}
