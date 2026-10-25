//! `Info.plist` rendering (pure).

/// Owner-approved bundle identifier (overridable via `--bundle-id` / `VEXEL_BUNDLE_ID`).
pub const BUNDLE_ID: &str = "io.github.megasoft1978.Vexel";
pub const MIN_MACOS: &str = "13.0";
pub const EXECUTABLE: &str = "vexel";
/// Icon base name: `CFBundleIconFile` and `Contents/Resources/<name>.icns`.
pub const ICON_NAME: &str = "AppIcon";

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

/// Renders the bundle `Info.plist`.
#[cfg(test)]
pub fn render_info_plist(version: &str, bundle_id: &str) -> String {
    render_info_plist_named(version, bundle_id, "Vexel")
}

pub fn render_info_plist_named(version: &str, bundle_id: &str, display_name: &str) -> String {
    let entries: Vec<(&str, String)> = vec![
        ("CFBundleDevelopmentRegion", string("en")),
        ("CFBundleExecutable", string(EXECUTABLE)),
        ("CFBundleIconFile", string(ICON_NAME)),
        ("CFBundleIdentifier", string(bundle_id)),
        ("CFBundleInfoDictionaryVersion", string("6.0")),
        ("CFBundleName", string(display_name)),
        ("CFBundlePackageType", string("APPL")),
        ("CFBundleShortVersionString", string(version)),
        ("CFBundleVersion", string(version)),
        ("LSMinimumSystemVersion", string(MIN_MACOS)),
        ("NSHighResolutionCapable", "<true/>".to_string()),
    ];
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
    fn key_set_is_exact() {
        let plist = render_info_plist("1.2.3", BUNDLE_ID);
        assert_eq!(
            keys(&plist),
            [
                "CFBundleDevelopmentRegion",
                "CFBundleExecutable",
                "CFBundleIconFile",
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
        let plist = render_info_plist("0.1.0", BUNDLE_ID);
        for expected in [
            "<key>CFBundleExecutable</key>\n\t<string>vexel</string>",
            "<key>CFBundleIconFile</key>\n\t<string>AppIcon</string>",
            "<key>CFBundleIdentifier</key>\n\t<string>io.github.megasoft1978.Vexel</string>",
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
    fn icon_key_names_the_bundled_icon_without_extension() {
        let plist = render_info_plist("1", BUNDLE_ID);
        assert!(plist.contains("<key>CFBundleIconFile</key>\n\t<string>AppIcon</string>"));
        assert!(!plist.contains("AppIcon.icns"));
    }

    #[test]
    fn values_are_xml_escaped() {
        assert_eq!(escape("a&b<c>\"d'"), "a&amp;b&lt;c&gt;&quot;d&apos;");
        let plist = render_info_plist("1.0 <beta> & \"rc\"", "a&b");
        assert!(plist.contains("<string>1.0 &lt;beta&gt; &amp; &quot;rc&quot;</string>"));
        assert!(plist.contains("<string>a&amp;b</string>"));
        assert!(!plist.contains("<beta>"));
    }
}
