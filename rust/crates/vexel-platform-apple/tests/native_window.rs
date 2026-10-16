use vexel_platform_apple::Modifiers;

#[test]
fn cocoa_modifier_flags_map_to_vexel_ui_order() {
    let flags = (1 << 17) | (1 << 18) | (1 << 19) | (1 << 20);
    assert_eq!(Modifiers::from_native_bits(flags).0, 0b1111);
    for (native, ui) in [(1 << 17, 1), (1 << 20, 2), (1 << 19, 4), (1 << 18, 8)] {
        assert_eq!(Modifiers::from_native_bits(native).0, ui);
    }
}
