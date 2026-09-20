//! Experiment 65a: a no_std Rust E32Main through symdev's recorded link, native
//! elf2e32, native SIS and EKA2L1. Stand-in target armv5te-unknown-linux-gnueabi
//! (prebuilt core; same CPU/ABI object code; target_os lies and is never consulted).
#![no_std]
#![no_main]

/// The shape of a `_LIT16`: observed from lit.o (`02000000 48006900 0000`): a length
/// word with type nibble 0, then UTF-16 code units. euser reads it as `const TDesC16&`.
#[repr(C)]
struct Lit16<const N: usize> {
    type_length: u32,
    buf: [u16; N],
}

const fn lit<const N: usize>(s: &[u8; N]) -> Lit16<N> {
    let mut buf = [0u16; N];
    let mut i = 0;
    while i < N {
        buf[i] = s[i] as u16;
        i += 1;
    }
    Lit16 { type_length: N as u32, buf }
}

static HELLO: Lit16<19> = lit(b"Hello from Rust SDK");

unsafe extern "C" {
    // nm -D euser.dso: static member functions, no `this`, plain EABI.
    #[link_name = "_ZN4User9InfoPrintERK7TDesC16"]
    fn user_info_print(des: *const Lit16<19>) -> i32;
    #[link_name = "_ZN4User5AfterE27TTimeIntervalMicroSeconds32"]
    fn user_after(micros: i32);
    #[link_name = "_ZN4User4ExitEi"]
    fn user_exit(reason: i32) -> !;
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    unsafe { user_exit(-1) }
}

/// eexe.lib's startup calls the C++-mangled `E32Main()`.
#[unsafe(export_name = "_Z7E32Mainv")]
pub extern "C" fn e32main() -> i32 {
    unsafe {
        user_info_print(&HELLO);
        user_after(5_000_000);
    }
    0
}
