//! @module cu_20261001_006
//! @description Regression test (header balance): the acer logo and the gear/close buttons are centred on the middle of
//! the title block (NITROSENSE + strapline, ink 17-54 DIP, centre 35.5), not on the wordmark alone, which left
//! them visibly high. Measured by ink in the 2x render: logo 35.25, gear 35.5, close 35.75.
const CSS: &str = include_str!("../../src/ui/web/css/popup.css");

#[test]
fn logo_and_buttons_sit_on_the_title_block_centre() {
    assert!(CSS.contains("\n.brand { position: absolute; left: 25px; top: 16px;"), "logo");
    assert!(CSS.contains("\n.hdr-actions { position: absolute; top: 23.5px;"), "gear and close");
    assert!(CSS.contains("\n.title { position: absolute; left: 0; right: 0; top: 11px;"), "the titles they centre on");
}
