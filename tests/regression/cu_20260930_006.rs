//! @module cu_20260930_006
//! @description Regression test: the production pipe name is the valid local path `\\.\pipe\NitroTray`
//! (a heredoc once collapsed it to `\.\pipe\NitroTray`, and the test built the same way agreed).
use nitrotray::meta::PIPE_NAME;

#[test]
#[allow(clippy::byte_char_slices, reason = "explicit bytes stay correct even if escapes are mangled")]
fn pipe_name_is_byte_exact() {
    let bytes = PIPE_NAME.as_bytes();
    assert_eq!(&bytes[..4], &[b'\\', b'\\', b'.', b'\\']);
    assert_eq!(&PIPE_NAME[4..], "pipe\\NitroTray");
    assert_eq!(PIPE_NAME.len(), 18);
}
