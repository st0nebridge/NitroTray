//! @module raster_test
//! @description Canvas fills, blending, anti-aliased lines and even-odd polygons.
use nitrotray::tray::raster::{Canvas, Rgba};

#[test]
fn parses_hex_colours() {
    assert_eq!(Rgba::from_hex("#FF252D"), Some(Rgba(255, 37, 45, 255)));
    assert_eq!(Rgba::from_hex("#00ff00"), Some(Rgba(0, 255, 0, 255)));
    for bad in ["FF252D", "#FF25", "#GG0000", "#ÿÿÿÿÿÿ"] {
        assert_eq!(Rgba::from_hex(bad), None, "{bad}");
    }
    assert_eq!(Rgba(1, 2, 3, 4).with_alpha(9), Rgba(1, 2, 3, 9));
}

#[test]
fn fill_and_pixel_access() {
    let mut c = Canvas::new(4, 3);
    assert_eq!((c.width(), c.height()), (4, 3));
    c.fill(Rgba(10, 20, 30, 255));
    assert_eq!(c.pixel(3, 2), Rgba(10, 20, 30, 255));
    assert_eq!(c.into_rgba().len(), 4 * 3 * 4);
}

#[test]
fn blending_is_source_over() {
    let mut c = Canvas::new(2, 1);
    c.fill(Rgba(0, 0, 0, 255));
    c.blend(0, 0, Rgba(255, 0, 0, 255), 0.5);
    let p = c.pixel(0, 0);
    assert!((126..=129).contains(&p.0) && p.3 == 255, "{p:?}");
    c.blend(1, 0, Rgba(255, 255, 255, 255), 0.0);
    assert_eq!(c.pixel(1, 0), Rgba(0, 0, 0, 255), "zero coverage is a no-op");
    c.blend(-1, 0, Rgba(255, 255, 255, 255), 1.0);
    c.blend(5, 5, Rgba(255, 255, 255, 255), 1.0);
    let mut t = Canvas::new(1, 1);
    t.blend(0, 0, Rgba(0, 0, 255, 128), 1.0);
    assert_eq!(t.pixel(0, 0), Rgba(0, 0, 255, 128), "onto transparent keeps colour");
}

#[test]
fn column_fill_is_antialiased_at_the_top() {
    let mut c = Canvas::new(1, 4);
    c.fill_column_from(0, 1.5, Rgba(255, 255, 255, 255));
    assert_eq!(c.pixel(0, 0).3, 0);
    assert!((120..=135).contains(&c.pixel(0, 1).3));
    assert_eq!(c.pixel(0, 2).3, 255);
    assert_eq!(c.pixel(0, 3).3, 255);
}

#[test]
fn horizontal_line_covers_its_row() {
    let mut c = Canvas::new(8, 5);
    c.line((0.5, 2.5), (7.5, 2.5), 1.0, Rgba(0, 255, 0, 255));
    for x in 0..8 {
        assert!(c.pixel(x, 2).3 > 200, "x={x}");
        assert_eq!(c.pixel(x, 0).3, 0);
    }
    let mut dot = Canvas::new(3, 3);
    dot.line((1.5, 1.5), (1.5, 1.5), 1.0, Rgba(255, 0, 0, 255));
    assert!(dot.pixel(1, 1).3 > 200 && dot.pixel(0, 0).3 < 60);
}

#[test]
fn even_odd_polygons_cut_holes() {
    let mut c = Canvas::new(10, 10);
    let outer = vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
    let inner = vec![(3.0, 3.0), (7.0, 3.0), (7.0, 7.0), (3.0, 7.0)];
    c.fill_polygons(&[outer, inner], Rgba(255, 0, 0, 255));
    assert_eq!(c.pixel(1, 1).3, 255);
    assert_eq!(c.pixel(5, 5).3, 0, "hole");
    let mut half = Canvas::new(2, 2);
    half.fill_polygons(&[vec![(0.0, 0.0), (1.5, 0.0), (1.5, 2.0), (0.0, 2.0)]], Rgba(255, 255, 255, 255));
    assert!((110..=145).contains(&half.pixel(1, 0).3), "partial coverage anti-aliases");
}
