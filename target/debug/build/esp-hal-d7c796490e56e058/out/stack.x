
SECTIONS {
  /* must be last segment using RWDATA */
  .stack (NOLOAD) : ALIGN(4)
  {
    . = ALIGN (4);
    _stack_end = ABSOLUTE(.);
    _stack_end_cpu0 = ABSOLUTE(.);
  } > RWDATA
}

PROVIDE(_stack_start = ORIGIN(RWDATA) + LENGTH(RWDATA));
PROVIDE(_stack_start_cpu0 = ORIGIN(RWDATA) + LENGTH(RWDATA));


/*
Provide the stack_guard for `stack-protector`

Ideally the offset should be configurable - should be done once we have https://github.com/esp-rs/esp-hal/issues/1111
*/
PROVIDE(__stack_chk_guard = _stack_end + 4096);
