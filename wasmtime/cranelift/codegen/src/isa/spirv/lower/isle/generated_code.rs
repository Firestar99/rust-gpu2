#![expect(
    dead_code,
    unreachable_patterns,
    unused_imports,
    unused_variables,
    irrefutable_let_patterns,
    clippy::clone_on_copy,
    missing_docs,
    reason = "generated code"
)]

include!(concat!(env!("ISLE_DIR"), "/isle_spirv.rs"));
