use std::fs;
use std::path::{Path, PathBuf};

use crate::codegen::OperatingSystem;

/// Default embedded Apple ICNS icon (Alya Application icon) for standalone zero-dependency bundling.
const DEFAULT_APP_ICON_ICNS: &[u8] = include_bytes!("../../assets/brand/icons/alya-app-dark.icns");

/// Target platform for a bundle. `BundleOptions::new` defaults to macOS
/// (historical behavior of `--bundle`); callers set the real target
/// explicitly for Windows/Linux output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BundleOs {
    Windows,
    #[default]
    MacOs,
    Linux,
}

impl From<OperatingSystem> for BundleOs {
    fn from(os: OperatingSystem) -> Self {
        match os {
            OperatingSystem::Windows => BundleOs::Windows,
            OperatingSystem::MacOS => BundleOs::MacOs,
            _ => BundleOs::Linux,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BundleOptions {
    pub app_name: String,
    pub bundle_dir: PathBuf,
    pub bundle_id: Option<String>,
    pub bundle_version: Option<String>,
    pub icon_path: Option<String>,
    pub os: BundleOs,
    /// GUI application: enables PerMonitorV2 DPI awareness (Windows
    /// manifest) and desktop integration hints (Linux `.desktop`).
    pub gui: bool,
}

impl BundleOptions {
    pub fn new(app_name: &str, output_override: Option<&str>) -> Self {
        let bundle_name = if let Some(out) = output_override {
            if out.ends_with(".app") {
                out.to_string()
            } else {
                format!("{}.app", out)
            }
        } else {
            format!("{}.app", app_name)
        };

        let stem = Path::new(&bundle_name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(app_name)
            .to_string();

        Self {
            app_name: stem,
            bundle_dir: PathBuf::from(bundle_name),
            bundle_id: None,
            bundle_version: None,
            icon_path: None,
            os: BundleOs::MacOs,
            gui: false,
        }
    }

    /// Sets the target platform, adjusting the bundle directory suffix for
    /// macOS (`.app`) and leaving plain directory names elsewhere.
    pub fn with_os(mut self, os: BundleOs) -> Self {
        self.os = os;
        if os != BundleOs::MacOs && self.bundle_dir.extension().is_some_and(|e| e == "app") {
            self.bundle_dir = self.bundle_dir.with_extension("");
        }
        self
    }

    /// Marks the bundle as a GUI application (DPI awareness, desktop hints).
    pub fn with_gui(mut self, gui: bool) -> Self {
        self.gui = gui;
        self
    }

    pub fn binary_path(&self) -> PathBuf {
        match self.os {
            BundleOs::MacOs => self
                .bundle_dir
                .join("Contents")
                .join("MacOS")
                .join(&self.app_name),
            BundleOs::Windows => self.bundle_dir.join(format!("{}.exe", self.app_name)),
            BundleOs::Linux => self.bundle_dir.join(&self.app_name),
        }
    }

    /// Creates the on-disk bundle layout for the target platform and writes
    /// all generated metadata (Info.plist / manifest / .desktop / icon).
    pub fn create_structure(&self) -> Result<(), String> {
        match self.os {
            BundleOs::MacOs => self.create_macos_structure(),
            BundleOs::Windows => self.create_windows_structure(),
            BundleOs::Linux => self.create_linux_structure(),
        }
    }

    fn create_macos_structure(&self) -> Result<(), String> {
        let macos_dir = self.bundle_dir.join("Contents").join("MacOS");
        let resources_dir = self.bundle_dir.join("Contents").join("Resources");

        fs::create_dir_all(&macos_dir)
            .map_err(|e| format!("Failed to create bundle directory {:?}: {}", macos_dir, e))?;
        fs::create_dir_all(&resources_dir).map_err(|e| {
            format!(
                "Failed to create bundle directory {:?}: {}",
                resources_dir, e
            )
        })?;

        // Write Info.plist
        let info_plist = self.generate_info_plist();
        let plist_path = self.bundle_dir.join("Contents").join("Info.plist");
        fs::write(&plist_path, info_plist)
            .map_err(|e| format!("Failed to write Info.plist at {:?}: {}", plist_path, e))?;

        // Install AppIcon.icns
        self.install_icon()?;

        Ok(())
    }

    fn create_windows_structure(&self) -> Result<(), String> {
        fs::create_dir_all(&self.bundle_dir).map_err(|e| {
            format!(
                "Failed to create bundle directory {:?}: {}",
                self.bundle_dir, e
            )
        })?;

        // Side-by-side application manifest (DPI awareness, UAC, version).
        let manifest = self.generate_windows_manifest();
        let manifest_path = self
            .bundle_dir
            .join(format!("{}.exe.manifest", self.app_name));
        fs::write(&manifest_path, manifest)
            .map_err(|e| format!("Failed to write manifest at {:?}: {}", manifest_path, e))?;

        Ok(())
    }

    fn create_linux_structure(&self) -> Result<(), String> {
        fs::create_dir_all(&self.bundle_dir).map_err(|e| {
            format!(
                "Failed to create bundle directory {:?}: {}",
                self.bundle_dir, e
            )
        })?;

        // XDG desktop entry pointing at the bundled binary.
        let desktop = self.generate_desktop_file();
        let desktop_path = self.bundle_dir.join(format!("{}.desktop", self.app_name));
        fs::write(&desktop_path, desktop)
            .map_err(|e| format!("Failed to write desktop entry at {:?}: {}", desktop_path, e))?;

        Ok(())
    }

    /// Windows side-by-side application manifest: asInvoker UAC, version
    /// metadata, and (for GUI apps) PerMonitorV2 DPI awareness.
    pub fn generate_windows_manifest(&self) -> String {
        // SxS requires exactly four dot-separated parts ("1.0.0.0").
        let version = {
            let mut parts: Vec<&str> = self
                .bundle_version
                .as_deref()
                .unwrap_or("1.0.0")
                .split('.')
                .collect();
            while parts.len() < 4 {
                parts.push("0");
            }
            parts[..4].join(".")
        };
        let dpi_block = if self.gui {
            r#"    <application xmlns="urn:schemas-microsoft-com:asm.v3">
      <windowsSettings>
        <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/PM</dpiAware>
        <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
      </windowsSettings>
    </application>
"#
            .to_string()
        } else {
            String::new()
        };
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity version="{version}" processorArchitecture="*" name="{name}" type="win32"/>
  <description>{name}</description>
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="asInvoker" uiAccess="false"/>
      </requestedPrivileges>
    </security>
  </trustInfo>
{dpi}</assembly>
"#,
            version = version,
            name = self.app_name,
            dpi = dpi_block
        )
    }

    /// XDG desktop entry launching the bundled binary.
    pub fn generate_desktop_file(&self) -> String {
        let categories = if self.gui {
            "Utility;Graphics;"
        } else {
            "Utility;"
        };
        // XDG paths always use forward slashes, even when the bundle is
        // authored on Windows.
        let exec = self.binary_path().display().to_string().replace('\\', "/");
        format!(
            "[Desktop Entry]\nType=Application\nName={name}\nExec={bin}\nIcon={name}\nCategories={cats}\nTerminal=false\n",
            name = self.app_name,
            bin = exec,
            cats = categories
        )
    }

    pub fn generate_info_plist(&self) -> String {
        let id = self.bundle_id.clone().unwrap_or_else(|| {
            let clean_name = self.app_name.to_lowercase().replace(' ', "-");
            format!("com.alya.{}", clean_name)
        });
        let version = self.bundle_version.as_deref().unwrap_or("1.0.0");

        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>{}</string>
    <key>CFBundleIdentifier</key>
    <string>{}</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>{}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>{}</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
"#,
            self.app_name, id, self.app_name, version
        )
    }

    fn install_icon(&self) -> Result<(), String> {
        let target_icon = self
            .bundle_dir
            .join("Contents")
            .join("Resources")
            .join("AppIcon.icns");

        if let Some(ref custom_icon) = self.icon_path {
            if Path::new(custom_icon).exists() {
                fs::copy(custom_icon, &target_icon)
                    .map_err(|e| format!("Failed to copy icon from '{}': {}", custom_icon, e))?;
                return Ok(());
            } else {
                return Err(format!("Icon file not found: {}", custom_icon));
            }
        }

        // Try local asset file first if available, otherwise write embedded ICNS
        let local_candidates = [
            "assets/brand/icons/alya-app-dark.icns",
            "../assets/brand/icons/alya-app-dark.icns",
        ];

        for candidate in &local_candidates {
            if Path::new(candidate).exists() && fs::copy(candidate, &target_icon).is_ok() {
                return Ok(());
            }
        }

        // Write embedded fallback ICNS
        fs::write(&target_icon, DEFAULT_APP_ICON_ICNS)
            .map_err(|e| format!("Failed to write default AppIcon.icns: {}", e))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bundle_options_and_structure() {
        let temp_dir =
            std::env::temp_dir().join(format!("alya_test_bundle_{}", std::process::id()));
        let bundle_name = temp_dir.join("TestApp.app");
        let opts = BundleOptions::new("TestApp", Some(bundle_name.to_str().unwrap()));

        assert_eq!(opts.app_name, "TestApp");
        assert_eq!(opts.bundle_dir, bundle_name);
        assert_eq!(
            opts.binary_path(),
            bundle_name.join("Contents").join("MacOS").join("TestApp")
        );

        let res = opts.create_structure();
        assert!(res.is_ok(), "create_structure failed: {:?}", res);

        assert!(bundle_name.join("Contents").join("Info.plist").exists());
        assert!(bundle_name
            .join("Contents")
            .join("Resources")
            .join("AppIcon.icns")
            .exists());
        assert!(bundle_name.join("Contents").join("MacOS").exists());

        let plist_content =
            fs::read_to_string(bundle_name.join("Contents").join("Info.plist")).unwrap();
        assert!(plist_content.contains("<string>TestApp</string>"));
        assert!(plist_content.contains("<string>com.alya.testapp</string>"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_bundle_windows_manifest_and_structure() {
        let temp_dir =
            std::env::temp_dir().join(format!("alya_test_winbundle_{}", std::process::id()));
        let bundle_dir = temp_dir.join("WinApp");
        let opts = BundleOptions::new("WinApp", Some(bundle_dir.to_str().unwrap()))
            .with_os(BundleOs::Windows)
            .with_gui(true);

        assert_eq!(
            opts.binary_path(),
            bundle_dir.join("WinApp.exe"),
            "windows binary lives next to the manifest"
        );

        let manifest = opts.generate_windows_manifest();
        assert!(
            manifest.contains("PerMonitorV2"),
            "gui manifest has DPI awareness"
        );
        assert!(manifest.contains("asInvoker"), "manifest has UAC level");
        assert!(
            manifest.contains("version=\"1.0.0.0\""),
            "manifest has dotted quad version, got:\n{}",
            manifest
        );

        let plain = BundleOptions::new("Tool", None).with_os(BundleOs::Windows);
        assert!(
            !plain.generate_windows_manifest().contains("PerMonitorV2"),
            "non-gui manifest omits DPI block"
        );

        assert!(opts.create_structure().is_ok());
        assert!(bundle_dir.join("WinApp.exe.manifest").exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_bundle_linux_desktop_and_structure() {
        let temp_dir =
            std::env::temp_dir().join(format!("alya_test_lnxbundle_{}", std::process::id()));
        let bundle_dir = temp_dir.join("LnxApp");
        let opts = BundleOptions::new("LnxApp", Some(bundle_dir.to_str().unwrap()))
            .with_os(BundleOs::Linux)
            .with_gui(true);

        assert_eq!(
            opts.binary_path(),
            bundle_dir.join("LnxApp"),
            "linux binary lives in the bundle dir"
        );

        let desktop = opts.generate_desktop_file();
        assert!(desktop.contains("Type=Application"));
        assert!(desktop.contains("Name=LnxApp"));
        assert!(
            desktop.contains("Graphics;"),
            "gui desktop has Graphics category"
        );
        assert!(
            !desktop.contains('\\'),
            "desktop paths use forward slashes, got:\n{}",
            desktop
        );

        assert!(opts.create_structure().is_ok());
        assert!(bundle_dir.join("LnxApp.desktop").exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_bundle_os_suffix_handling() {
        // macOS keeps `.app`; other platforms strip it.
        let mac = BundleOptions::new("App", Some("out/App.app"));
        assert_eq!(mac.bundle_dir, PathBuf::from("out/App.app"));

        let win = BundleOptions::new("App", Some("out/App.app")).with_os(BundleOs::Windows);
        assert_eq!(win.bundle_dir, PathBuf::from("out/App"));

        let lnx = BundleOptions::new("App", None).with_os(BundleOs::Linux);
        assert_eq!(lnx.bundle_dir, PathBuf::from("App"));
    }
}
