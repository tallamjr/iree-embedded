// Proves the FFI layer is wired up end to end:
//  1. the cc-compiled shim for IREE's static-inline helpers works
//     (`iree_make_cstring_view` computes a string view), and
//  2. the unified runtime archive links (`iree_status_free` is a real exported
//     symbol from it).
// Full instance creation is exercised in the safe crate once the Arena
// allocator exists (the safe API never uses the system allocator).
#[test]
fn runtime_links_and_wrappers_work() {
    use iree_embedded_sys as sys;
    unsafe {
        let name = b"hello\0";
        let sv = sys::iree_make_cstring_view(name.as_ptr() as *const core::ffi::c_char);
        assert_eq!(sv.size, 5, "static-inline shim produced wrong length");

        // IREE_STATUS_OK is the zero/null status value.
        let ok: sys::iree_status_t = core::ptr::null_mut();
        sys::iree_status_free(ok); // exported from the unified archive
    }
}
