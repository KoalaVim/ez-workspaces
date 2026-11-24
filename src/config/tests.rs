use super::model::{EzConfig, NativeConfig, SelectorBackend};

#[test]
fn backend_parse_accepts_known_values() {
    assert_eq!(SelectorBackend::parse("fzf").unwrap(), SelectorBackend::Fzf);
    assert_eq!(
        SelectorBackend::parse("native").unwrap(),
        SelectorBackend::Native
    );
}

#[test]
fn backend_parse_rejects_unknown_listing_accepted() {
    let msg = SelectorBackend::parse("skim").unwrap_err().to_string();
    assert!(msg.contains("skim"), "{msg}");
    assert!(msg.contains("fzf") && msg.contains("native"), "{msg}");
}

#[test]
fn native_defaults_when_section_missing() {
    let cfg: EzConfig = toml::from_str("").unwrap();
    assert_eq!(cfg.native, NativeConfig::default());
    assert_eq!(cfg.native.preview_ratio, 50);
    assert!(cfg.native.snapshot);
}

#[test]
fn native_section_round_trips() {
    let mut cfg = EzConfig::default();
    cfg.native.preview_ratio = 35;
    cfg.native.snapshot = false;
    let text = toml::to_string_pretty(&cfg).unwrap();
    assert!(text.contains("[native]"));
    let back: EzConfig = toml::from_str(&text).unwrap();
    assert_eq!(back.native.preview_ratio, 35);
    assert!(!back.native.snapshot);
}

#[test]
fn native_partial_section_uses_defaults() {
    let cfg: EzConfig = toml::from_str("[native]\npreview_ratio = 60\n").unwrap();
    assert_eq!(cfg.native.preview_ratio, 60);
    assert!(cfg.native.snapshot);
}
