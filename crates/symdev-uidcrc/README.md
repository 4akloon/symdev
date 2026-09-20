# symdev-uidcrc

The Symbian UID checksum: the fourth word of the 16-byte UID block that starts every E32 image,
`.rsc` resource file and SIS file, computed from UID1–UID3. Native replacement for the SDK's
`uidcrc.exe`.

## Library

```rust
use symdev_uidcrc::UidCrc;

let crc = UidCrc::new(0x10000079, 0x100039ce, 0xef9f2cab);
crc.checked();   // 0xd7edaf52
crc.line();      // "0x10000079 0x100039ce 0xef9f2cab 0xd7edaf52"
crc.bytes();     // the 16 bytes: three UIDs and the checksum, little-endian
UidCrc::parse_uid("0xef9f2cab");   // Some(0xef9f2cab); decimal also accepted
```

`UidCrcTool` builds the argv for running the original `uidcrc.exe` under Wine. It exists so the
native result can be compared with the original tool; nothing in the normal pipeline uses it.

## Binary

```bash
uidcrc 0x10000079 0x100039ce 0xef9f2cab            # prints the line above
uidcrc 0x10000079 0x100039ce 0xef9f2cab uid.bin    # writes the 16 bytes to a file
```

## Testing

```bash
cargo test -p symdev-uidcrc --offline
```
