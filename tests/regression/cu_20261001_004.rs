//! @module cu_20261001_004
//! @description Regression test (popup alignment): the popup sits on one horizontal grid — 16 DIP margins, two
//! equal columns with a 12 DIP gutter for cards and footer, one chevron size and inset per column, and no
//! per-GPU or per-button offset overrides (those reproduced the reference image's own misalignments),
//! room under the header strapline, and footer labels not lifted off their buttons' centre.
//! Positions were measured in the rendered page; this pins the rules that produce them.
const CSS: &str = include_str!("../../src/ui/web/css/popup.css");

fn rule<'a>(selector: &str) -> &'a str {
    let start = CSS.find(&format!("\n{selector} {{")).unwrap_or_else(|| panic!("missing rule {selector}"));
    let body = &CSS[start..];
    &body[..body.find('}').unwrap()]
}

#[test]
fn rows_share_the_16_dip_margins() {
    for selector in [".fans", ".compute"] {
        let r = rule(selector);
        assert!(r.contains("left: 16px") && r.contains("width: 540px"), "{selector}: {r}");
    }
    for selector in [".controls", ".ftr", ".rule"] {
        let r = rule(selector);
        assert!(r.contains("left: 16px") && r.contains("right: 16px"), "{selector}: {r}");
    }
}

#[test]
fn cards_and_footer_use_two_equal_columns() {
    let controls = rule(".controls");
    assert!(controls.contains("grid-template-columns: 1fr 1fr") && controls.contains("column-gap: 12px"));
    let footer = rule(".ftr");
    assert!(footer.contains("grid-template-columns: 1fr 12px 1fr"), "divider on the column split: {footer}");
}

#[test]
fn no_column_specific_offsets_remain() {
    for override_rule in [
        "#proc-gpu h2 {",
        "#proc-gpu .temp {",
        "#proc-gpu .sub {",
        "#proc-gpu .sub .pipe {",
        "#btn-quick {",
        "#btn-monitor .chev {",
        "#btn-quick .chev {",
        "#fan-control .ctl-head .chev {",
        ".fan + .vline + .fan {",
        ".fan + .vline + .fan .fan-text {",
    ] {
        assert!(!CSS.contains(override_rule), "{override_rule} reintroduces a per-column offset");
    }
}

#[test]
fn every_chevron_has_the_same_size() {
    for selector in [".fan .chev", ".ctl-head .chev", ".ftr-btn .chev"] {
        let r = rule(selector);
        assert!(r.contains("width: 20.5px") && r.contains("height: 20.5px"), "{selector}: {r}");
    }
}

#[test]
fn header_leaves_room_under_the_strapline() {
    assert!(rule(".hdr").contains("height: 65px"), "rule 4 DIP below the old 62");
    assert!(rule(".title").contains("top: 11px"), "title 2 DIP higher than the old 13");
    assert!(rule(".fans").contains("top: 79px"), "rows below follow the header");
}

#[test]
fn footer_labels_are_centred_on_their_buttons() {
    // Measured: a 1 DIP lift puts the 13.2 px labels' cap-height centre on the buttons' centre; 2 DIP was high.
    assert!(rule(".ftr-btn > span:nth-child(2)").contains("transform: translateY(-1px)"));
    assert!(rule("#btn-quick .ftr-icon").contains("transform: translateY(-1px)"));
}
