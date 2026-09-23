use std::fs;
use std::path::{Path, PathBuf};

use crate::codegen::OperatingSystem;

/// Default embedded Apple ICNS icon (Alya Application icon) for standalone zero-dependency bundling.
const DEFAULT_APP_ICON_ICNS: &[u8] = include_bytes!("../../assets/brand/icons/alya-app-dark.icns");
/// Default embedded Windows ICO icon for standalone bundling.
const DEFAULT_APP_ICON_ICO: &[u8] = include_bytes!("../../assets/brand/icons/alya-app-dark.ico");
/// Default embedded PNG icon for Linux desktop entries.
const DEFAULT_APP_ICON_PNG: &[u8] = include_bytes!("../../assets/brand/icons/alya-app-dark.png");

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

        // Stage the icon (.ico) and resource script; the .res is compiled
        // at link time via `build_windows_icon_resource` (needs windres).
        self.stage_windows_icon()?;
        let rc_path = self.bundle_dir.join(format!("{}.rc", self.app_name));
        fs::write(&rc_path, self.generate_windows_rc())
            .map_err(|e| format!("Failed to write resource script at {:?}: {}", rc_path, e))?;

        Ok(())
    }

    /// Resolves the Windows icon: copies a user-supplied `.ico` or writes
    /// the embedded default into the bundle directory. Returns the path.
    fn stage_windows_icon(&self) -> Result<PathBuf, String> {
        let target = self.bundle_dir.join(format!("{}.ico", self.app_name));
        if let Some(ref custom) = self.icon_path {
            let src = Path::new(custom);
            if !src.exists() {
                return Err(format!("Icon file not found: {}", custom));
            }
            let is_ico = src
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("ico"));
            if !is_ico {
                return Err(format!(
                    "Windows bundles need a .ico file (got '{}'). Convert it first or omit --icon.",
                    custom
                ));
            }
            fs::copy(src, &target)
                .map_err(|e| format!("Failed to copy icon from '{}': {}", custom, e))?;
        } else {
            fs::write(&target, DEFAULT_APP_ICON_ICO)
                .map_err(|e| format!("Failed to write default icon: {}", e))?;
        }
        Ok(target)
    }

    /// Resource script embedding the staged icon as `IDI_ICON1`.
    pub fn generate_windows_rc(&self) -> String {
        format!("IDI_ICON1 ICON \"{}.ico\"\n", self.app_name)
    }

    /// Compiles the staged icon into a COFF `.res` via windres.
    ///
    /// Returns the `.res` path for linking, or `None` when windres is
    /// unavailable (the bundle still builds, without an embedded icon).
    pub fn build_windows_icon_resource(&self) -> Result<Option<PathBuf>, String> {
        if self.os != BundleOs::Windows {
            return Ok(None);
        }
        if Self::find_windres().is_none() {
            eprintln!(
                "warning: windres not found; Windows bundle will use the default executable icon"
            );
            return Ok(None);
        }
        let rc_name = format!("{}.rc", self.app_name);
        let res_name = format!("{}.res", self.app_name);
        // Run inside the bundle dir so the bare `"name.ico"` reference in
        // the .rc resolves to the staged icon.
        let status = std::process::Command::new("windres")
            .current_dir(&self.bundle_dir)
            .arg(&rc_name)
            .arg("-O")
            .arg("coff")
            .arg("-o")
            .arg(&res_name)
            .status()
            .map_err(|e| format!("Failed to run windres: {}", e))?;
        if !status.success() {
            return Err("windres failed to compile the icon resource".to_string());
        }
        let res_path = self.bundle_dir.join(&res_name);
        if res_path.metadata().map(|m| m.len()).unwrap_or(0) == 0 {
            return Err("windres produced an empty resource file".to_string());
        }
        Ok(Some(res_path))
    }

    /// Locates `windres` on PATH (ships with mingw-w64 toolchains).
    fn find_windres() -> Option<PathBuf> {
        let probe = std::process::Command::new("windres")
            .arg("--version")
            .output();
        match probe {
            Ok(out) if out.status.success() => Some(PathBuf::from("windres")),
            _ => None,
        }
    }

    fn create_linux_structure(&self) -> Result<(), String> {
        fs::create_dir_all(&self.bundle_dir).map_err(|e| {
            format!(
                "Failed to create bundle directory {:?}: {}",
                self.bundle_dir, e
            )
        })?;

        // Stage the icon and point the desktop entry at its absolute path.
        let icon_abs = self.stage_linux_icon()?;
        let desktop = self.generate_desktop_file(icon_abs.as_deref());
        let desktop_path = self.bundle_dir.join(format!("{}.desktop", self.app_name));
        fs::write(&desktop_path, desktop)
            .map_err(|e| format!("Failed to write desktop entry at {:?}: {}", desktop_path, e))?;

        Ok(())
    }

    /// Resolves the Linux icon: copies a user-supplied image or writes the
    /// embedded default PNG into the bundle directory. Returns its absolute
    /// path for the `Icon=` desktop key.
    fn stage_linux_icon(&self) -> Result<Option<String>, String> {
        if let Some(ref custom) = self.icon_path {
            let src = Path::new(custom);
            if !src.exists() {
                return Err(format!("Icon file not found: {}", custom));
            }
            let file_name = src
                .file_name()
                .ok_or_else(|| format!("Invalid icon path: {}", custom))?;
            let target = self.bundle_dir.join(file_name);
            fs::copy(src, &target)
                .map_err(|e| format!("Failed to copy icon from '{}': {}", custom, e))?;
            return Ok(Some(target.display().to_string()));
        }
        let target = self.bundle_dir.join(format!("{}.png", self.app_name));
        fs::write(&target, DEFAULT_APP_ICON_PNG)
            .map_err(|e| format!("Failed to write default icon: {}", e))?;
        Ok(Some(target.display().to_string()))
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
    ///
    /// `icon_abs` is the absolute icon path staged next to the binary
    /// (always `Some` from `create_linux_structure`).
    pub fn generate_desktop_file(&self, icon_abs: Option<&str>) -> String {
        let categories = if self.gui {
            "Utility;Graphics;"
        } else {
            "Utility;"
        };
        // XDG paths always use forward slashes, even when the bundle is
        // authored on Windows.
        let exec = self.binary_path().display().to_string().replace('\\', "/");
        let icon_line = match icon_abs {
            Some(p) => format!("Icon={}\n", p.replace('\\', "/")),
            None => String::new(),
        };
        format!(
            "[Desktop Entry]\nType=Application\nName={name}\nExec={bin}\n{icon}Categories={cats}\nTerminal=false\n",
            name = self.app_name,
            bin = exec,
            icon = icon_line,
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

        let desktop = opts.generate_desktop_file(Some("/tmp/LnxApp/LnxApp.png"));
        assert!(desktop.contains("Type=Application"));
        assert!(desktop.contains("Name=LnxApp"));
        assert!(
            desktop.contains("Graphics;"),
            "gui desktop has Graphics category"
        );
        assert!(
            desktop.contains("Icon=/tmp/LnxApp/LnxApp.png"),
            "desktop points at the staged icon, got:\n{}",
            desktop
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
