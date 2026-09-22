# Alya Language Specification: Negative Fixtures
#
# Each file here MUST FAIL to compile. The first line declares the contract:
#   # EXPECT-FAIL: <stage>: <message fragment>
# where <stage> is `lex`, `parse` or `check` (resolve + constants +
# type validation, mirroring `alya check`; codegen/execution stages get their
# own markers when later chapters need them).
#
# Harness: `tests/negative_spec_tests.rs` compiles every file and requires a
# failure whose message contains the declared fragment. A negative fixture
# that starts passing means either the compiler got stricter (update the
# expectation) or a regression made invalid code acceptable (fix the compiler).
#
# Naming: `<chapter>_<topic>.alya`, e.g. `00_invalid_binary_digit.alya`.
