/* symbian-lld.ld: the layout rust-lld gives a Symbian E32 program (experiments 109, 112).
   symdev passes it with -T on every rust-lld link (`RustSdk::lld_script`); GNU ld uses its
   built-in symbianelf script instead. elf2e32 copies the first executable PT_LOAD as code
   and the writable one as data, so everything the image runs from -- code, the import
   stubs, any PLT and GOT, read-only data, init_array, the exception tables -- is in ONE
   executable PT_LOAD at -Ttext; .data/.bss in the writable PT_LOAD at -Tdata; the
   dynamic-linking tables in a third, read-only PT_LOAD that elf2e32 never copies. The
   symbols eexe.lib/usrt2_2.lib expect (the names ARM's linker defines, which GNU ld's
   symbianelf script defines too) are provided hidden. MIT, part of the Rust SDK. */
PHDRS
{
  text    PT_LOAD FLAGS(5);
  data    PT_LOAD FLAGS(6);
  dyn     PT_LOAD FLAGS(4);
  dynamic PT_DYNAMIC FLAGS(4);
}
SECTIONS
{
  PROVIDE_HIDDEN("Image$$ER_RO$$Base" = ADDR(.text));
  .text : { *(.text .text.* .stub) *(.emb_text) *(.glue_7t) *(.glue_7) } :text
  .plt : { *(.plt) } :text
  .got : { *(.got.plt) *(.got) } :text
  PROVIDE_HIDDEN("Image$$ER_RO$$Limit" = .);
  .rodata : { *(.rodata .rodata.* .constdata .constdata.*) } :text
  .init_array : {
    PROVIDE_HIDDEN("SHT$$INIT_ARRAY$$Base" = .);
    KEEP(*(SORT(.init_array.*))) KEEP(*(.init_array))
    PROVIDE_HIDDEN("SHT$$INIT_ARRAY$$Limit" = .);
  } :text
  .ARM.extab : { *(.ARM.extab* .gnu.linkonce.armextab.*) } :text
  PROVIDE_HIDDEN(".ARM.exidx$$Base" = .);
  .ARM.exidx : { *(.ARM.exidx* .gnu.linkonce.armexidx.*) } :text
  PROVIDE_HIDDEN(".ARM.exidx$$Limit" = .);

  /* GNU ld always emits a writable PT_LOAD at -Tdata, even an empty one; the symbol keeps
     lld from discarding an empty .data where it can (elf2e32 accepts an lld ELF without
     one: experiment 112, rule (b)). */
  .data : { __symdev_data_start = .; *(.data .data.*) } :data
  .bss : { *(.bss .bss.* COMMON) } :data

  . = 0x10000000;
  .dynsym : { *(.dynsym) } :dyn
  .gnu.version : { *(.gnu.version) } :dyn
  .gnu.version_d : { *(.gnu.version_d) } :dyn
  .gnu.version_r : { *(.gnu.version_r) } :dyn
  .hash : { *(.hash) } :dyn
  .gnu.hash : { *(.gnu.hash) } :dyn
  .dynstr : { *(.dynstr) } :dyn
  .rel.dyn : { *(.rel.dyn) } :dyn
  .rel.plt : { *(.rel.plt) } :dyn
  .dynamic : { *(.dynamic) } :dyn :dynamic
}
