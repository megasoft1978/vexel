//! `Info.plist` rendering (pure).

/// Placeholder bundle identifier; the owner decides the final value.
pub const BUNDLE_ID: &str = "app.vexel.Vexel";
pub const MIN_MACOS: &str = "13.0";
pub const EXECUTABLE: &str = "vexel";

/// Escapes the five XML special characters.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            other => out.push(other),
        }
    }
    out
}

/// Renders the bundle `Info.plist`. `icon_file` is the file name inside
/// `Contents/Resources`, present only when the owner supplies an icon.
pub fn render_info_plist(version: &str, bundle_id: &str, icon_file: Option<&str>) -> String {
    let mut entries: Vec<(&str, String)> = vec![
        ("CFBundleDevelopmentRegion", string("en")),
        ("CFBundleExecutable", string(EXECUTABLE)),
        ("CFBundleIdentifier", string(bundle_id)),
        ("CFBundleInfoDictionaryVersion", string("6.0")),
        ("CFBundleName", string("Vexel")),
        ("CFBundlePackageType", string("APPL")),
        ("CFBundleShortVersionString", string(version)),
        ("CFBundleVersion", string(version)),
        ("LSMinimumSystemVersion", string(MIN_MACOS)),
        ("NSHighResolutionCapable", "<true/>".to_string()),
    ];
    if let Some(icon) = icon_file {
        entries.push(("CFBundleIconFile", string(icon)));
    }
    let mut out = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
<plist version=\"1.0\">\n<dict>\n",
    );
    for (key, value) in entries {
        out.push_str(&format!("\t<key>{key}</key>\n\t{value}\n"));
    }
    out.push_str("</dict>\n</plist>\n");
    out
}

fn string(value: &str) -> String {
    format!("<string>{}</string>", escape(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(plist: &str) -> Vec<&str> {
        plist
            .split("<key>")
            .skip(1)
            .map(|part| part.split("</key>").next().unwrap())
            .collect()
    }

    #[test]
    fn key_set_is_exact_without_icon() {
        let plist = render_info_plist("1.2.3", BUNDLE_ID, None);
        assert_eq!(
            keys(&plist),
            [
                "CFBundleDevelopmentRegion",
                "CFBundleExecutable",
                "CFBundleIdentifier",
                "CFBundleInfoDictionaryVersion",
                "CFBundleName",
                "CFBundlePackageType",
                "CFBundleShortVersionString",
                "CFBundleVersion",
                "LSMinimumSystemVersion",
                "NSHighResolutionCapable",
            ]
        );
    }

    #[test]
    fn required_values_and_version_are_rendered() {
        let plist = render_info_plist("0.1.0", BUNDLE_ID, None);
        for expected in [
            "<key>CFBundleExecutable</key>\n\t<string>vexel</string>",
            "<key>CFBundleIdentifier</key>\n\t<string>app.vexel.Vexel</string>",
            "<key>CFBundleShortVersionString</key>\n\t<string>0.1.0</string>",
            "<key>LSMinimumSystemVersion</key>\n\t<string>13.0</string>",
            "<key>NSHighResolutionCapable</key>\n\t<true/>",
        ] {
            assert!(plist.contains(expected), "missing {expected}");
        }
        assert!(plist.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist"));
        assert!(plist.ends_with("</dict>\n</plist>\n"));
    }

    #[test]
    fn icon_key_appears_only_with_an_icon() {
        assert!(!render_info_plist("1", BUNDLE_ID, None).contains("CFBundleIconFile"));
        let with = render_info_plist("1", BUNDLE_ID, Some("Vexel.icns"));
        assert!(with.contains("<key>CFBundleIconFile</key>\n\t<string>Vexel.icns</string>"));
    }

    #[test]
    fn values_are_xml_escaped() {
        assert_eq!(escape("a&b<c>\"d'"), "a&amp;b&lt;c&gt;&quot;d&apos;");
        let plist = render_info_plist("1.0 <beta> & \"rc\"", "a&b", None);
        assert!(plist.contains("<string>1.0 &lt;beta&gt; &amp; &quot;rc&quot;</string>"));
        assert!(plist.contains("<string>a&amp;b</string>"));
        assert!(!plist.contains("<beta>"));
    }
}
