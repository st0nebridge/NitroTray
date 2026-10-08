//! @module cu_20261008_001
//! @description Regression test (build-machine paths in release binaries): the 0.1.1 executables embedded
//! 120 absolute paths into the Cargo home inside the builder's user profile (dependencies' panic
//! locations). Release builds remap the Cargo home and the checkout to neutral prefixes.

const BUILD: &str = include_str!("../../bin/build.ps1");

#[test]
fn release_builds_remap_the_cargo_home_and_the_checkout() {
    let release = &BUILD[BUILD.find("if (-not $Debug) {").expect("release branch")..];
    let release = &release[..release.find("\ncargo build").expect("cargo build after the release branch")];
    assert!(release.contains("\"--remap-path-prefix=$cargoHome=cargo\""), "Cargo home remapped");
    assert!(release.contains("--remap-path-prefix=$((Get-Location).Path)=nitrotray"), "checkout remapped");
    assert!(release.contains("$env:CARGO_HOME") && release.contains("$env:USERPROFILE '.cargo'"), "both Cargo homes");
    assert!(release.contains("$env:CARGO_ENCODED_RUSTFLAGS ="), "flags reach cargo intact, paths with spaces included");
}
