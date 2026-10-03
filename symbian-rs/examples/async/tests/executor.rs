//! `block_on` on the device, as a `cargo test` (symbian-test).
#![no_std]
#![no_main]

#[symbian_test::tests]
mod executor {
    use symbian_async::block_on;
    use symbian_test::{Evidence, ensure};

    #[test]
    fn block_on_returns_what_the_future_produced() -> Result<(), Evidence> {
        ensure(block_on(async { 42u32 }) == Ok(42), "block_on(async { 42 })")
    }
}
