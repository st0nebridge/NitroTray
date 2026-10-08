//! @module acer::nitrosense
//! @description Launches the full NitroSense application by its AppX user-model id.
//!
//! @input  NitroSense AUMID (default verified on this machine via Get-StartApps).
//! @output A `shell:AppsFolder` launch target, and (Windows) the launch side effect.
//! @dependencies windows (ShellExecuteW) on Windows

/// AUMID of NitroSense 3.1 as installed from the Acer Store package.
pub const DEFAULT_AUMID: &str = "AcerIncorporated.NitroSenseV31_48frkmn4z8aw4!App";

/// An AUMID is `<PackageFamilyName>!<AppId>`: restricted characters, exactly one `!`.
pub fn is_valid_aumid(aumid: &str) -> bool {
    let mut parts = aumid.split('!');
    let (Some(family), Some(app), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    let ok = |s: &str, extra: &[char]| {
        !s.is_empty() && s.len() <= 200 && s.chars().all(|c| c.is_ascii_alphanumeric() || extra.contains(&c))
    };
    ok(family, &['.', '_', '-']) && ok(app, &['.', '_'])
}

/// Shell target that activates the packaged app.
pub fn launch_target(aumid: &str) -> Option<String> {
    is_valid_aumid(aumid).then(|| format!("shell:AppsFolder\\{aumid}"))
}

/// Opens NitroSense; returns an error string when the AUMID is invalid or the shell refuses.
#[cfg(windows)]
pub fn launch(aumid: &str) -> Result<(), String> {
    let target = launch_target(aumid).ok_or_else(|| format!("invalid NitroSense app id '{aumid}'"))?;
    crate::platform::windows::shell_open(&target)
}
