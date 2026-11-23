//! ARC-6.5a: the handle may be shared between threads, so it is `Send` and `Sync`. The check
//! is at compile time: this file does not compile when the handle loses either trait.

use menura_core::Handle;

const fn assert_send_and_sync<T: Send + Sync>() {}

const _: () = assert_send_and_sync::<Handle>();

#[test]
fn arc_6_5a_the_handle_is_send_and_sync() {
    assert_send_and_sync::<Handle>();
}
