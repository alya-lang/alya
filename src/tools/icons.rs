//! Windows Explorer file icons for `.alya` sources.
//!
//! A VS Code extension can only theme icons *inside* the editor; the icon a
//! user sees in Windows Explorer comes from the OS file association. This
//! module implements the general (per-machine, opt-in) fix:
//!
//! ```text
//! alya icons install [--theme dark|light]   # register AlyaLang.alya ProgID
//! alya icons status                          # inspect current association
//! alya icons uninstall                       # restore previous association
//! ```
//!
//! Install materializes the embedded brand `.ico` into `~/.alya/icons/` and
//! registers a dedicated `AlyaLang.alya` ProgID (HKCU, no admin rights)
//! carrying the `DefaultIcon`. The *existing* open command is migrated into
//! the new ProgID, so double-click keeps opening the previously associated
//! app (usually VS Code); the previous ProgID is remembered for `uninstall`.
//! Registry access goes through `reg.exe` so no extra dependency is needed.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconTheme {
    Dark,
    Light,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IconsCommand {
    Status,
    Install { theme: IconTheme },
    Uninstall,
    Help,
}

/// Entry point from the driver. `Help` prints on every platform; the rest is
/// Windows-only.
pub fn run_icons(cmd: &IconsCommand) -> Result<(), String> {
    if *cmd == IconsCommand::Help {
        crate::cli::help::print_icons_help();
        return Ok(());
    }
    run_platform(cmd)
}

#[cfg(target_os = "windows")]
fn run_platform(cmd: &IconsCommand) -> Result<(), String> {
    match cmd {
        IconsCommand::Status => win::status(),
        IconsCommand::Install { theme } => win::install(*theme),
        IconsCommand::Uninstall => win::uninstall(),
        IconsCommand::Help => unreachable!("handled by run_icons"),
    }
}

#[cfg(not(target_os = "windows"))]
fn run_platform(_cmd: &IconsCommand) -> Result<(), String> {
    Err("Error: 'alya icons' is only supported on Windows.".to_string())
}

#[cfg(target_os = "windows")]
mod win {
    use super::IconTheme;
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;

    const PROG_ID: &str = "AlyaLang.alya";
    const PROG_FRIENDLY: &str = "Alya Source File";
    const EXTENSION: &str = ".alya";
    const ICON_FILE_NAME: &str = "alya-file.ico";
    const PREV_PROPID_VALUE: &str = "PrevProgId";

    const FILE_ICON_DARK: &[u8] = include_bytes!("../../assets/brand/icons/alya-file-dark.ico");
    const FILE_ICON_LIGHT: &[u8] = include_bytes!("../../assets/brand/icons/alya-file-light.ico");

    fn classes_key(suffix: &str) -> String {
        format!(r"HKCU\Software\Classes\{}", suffix)
    }

    fn merged_key(suffix: &str) -> String {
        format!(r"HKCR\{}", suffix)
    }

    fn prog_key() -> String {
        classes_key(PROG_ID)
    }

    fn default_icon_key() -> String {
        format!(r"{}\DefaultIcon", prog_key())
    }

    fn open_command_key(prog_id: &str) -> String {
        format!(r"HKCU\Software\Classes\{}\shell\open\command", prog_id)
    }

    fn merged_open_command_key(prog_id: &str) -> String {
        format!(r"HKCR\{}\shell\open\command", prog_id)
    }

    /// Parses `reg query ... /ve` (or `/v NAME`) output. Pure over text so it
    /// stays unit-testable without touching the registry.
    pub fn parse_reg_value(text: &str) -> Option<String> {
        for line in text.lines() {
            for marker in ["REG_SZ", "REG_EXPAND_SZ"] {
                if let Some(idx) = line.find(marker) {
                    let data = line[idx + marker.len()..].trim();
                    if data.is_empty() {
                        return None;
                    }
                    return Some(data.to_string());
                }
            }
        }
        None
    }

    fn reg_query_value(key: &str, name_args: &[&str]) -> Result<Option<String>, String> {
        let mut args = vec!["query", key];
        args.extend_from_slice(name_args);
        let output = Command::new("reg")
            .args(&args)
            .output()
            .map_err(|e| format!("Error: failed to run 'reg query': {}", e))?;
        if !output.status.success() {
            return Ok(None);
        }
        Ok(parse_reg_value(&String::from_utf8_lossy(&output.stdout)))
    }

    fn query_default(key: &str) -> Result<Option<String>, String> {
        reg_query_value(key, &["/ve"])
    }

    fn query_named(key: &str, name: &str) -> Result<Option<String>, String> {
        reg_query_value(key, &["/v", name])
    }

    fn reg_add(key: &str, name_args: &[&str], value: &str) -> Result<(), String> {
        let mut args = vec!["add", key];
        args.extend_from_slice(name_args);
        args.extend_from_slice(&["/t", "REG_SZ", "/d", value, "/f"]);
        let output = Command::new("reg")
            .args(&args)
            .output()
            .map_err(|e| format!("Error: failed to run 'reg add': {}", e))?;
        if output.status.success() {
            return Ok(());
        }
        Err(format!(
            "Error: 'reg add {}' failed: {}",
            key,
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }

    fn set_default(key: &str, value: &str) -> Result<(), String> {
        reg_add(key, &["/ve"], value)
    }

    fn set_named(key: &str, name: &str, value: &str) -> Result<(), String> {
        reg_add(key, &["/v", name], value)
    }

    fn delete_tree(key: &str) -> Result<(), String> {
        let output = Command::new("reg")
            .args(["delete", key, "/f"])
            .output()
            .map_err(|e| format!("Error: failed to run 'reg delete': {}", e))?;
        if output.status.success() {
            return Ok(());
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("unable to find") {
            return Ok(());
        }
        Err(format!(
            "Error: 'reg delete {}' failed: {}",
            key,
            stderr.trim()
        ))
    }

    fn delete_default(key: &str) -> Result<(), String> {
        let output = Command::new("reg")
            .args(["delete", key, "/ve", "/f"])
            .output()
            .map_err(|e| format!("Error: failed to run 'reg delete': {}", e))?;
        if output.status.success() {
            return Ok(());
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("unable to find") {
            return Ok(());
        }
        Err(format!(
            "Error: 'reg delete {}' failed: {}",
            key,
            stderr.trim()
        ))
    }

    fn refresh_icon_cache() {
        // Best effort: notify Explorer to repaint file icons.
        let _ = Command::new("ie4uinit.exe").arg("-show").output();
    }

    fn icons_dir() -> Result<PathBuf, String> {
        crate::driver::toolchain::get_global_alya_dir()
            .map(|d| d.join("icons"))
            .ok_or_else(|| "Error: cannot resolve home directory for ~/.alya/icons.".to_string())
    }

    fn current_progid() -> Result<Option<String>, String> {
        // HKCR merges HKCU/HKLM so machine-wide handlers are visible too.
        query_default(&merged_key(EXTENSION))
    }

    fn current_open_command(prog_id: &str) -> Result<Option<String>, String> {
        query_default(&merged_open_command_key(prog_id))
    }

    fn current_default_icon() -> Result<Option<String>, String> {
        query_default(&merged_key(&format!(r"{}\DefaultIcon", PROG_ID)))
    }

    pub fn status() -> Result<(), String> {
        let dir = icons_dir()?;
        let icon_path = dir.join(ICON_FILE_NAME);
        println!("Alya file icons (Windows Explorer)\n");
        match current_progid()? {
            Some(id) if id == PROG_ID => {
                println!("  .alya association : {} (managed by 'alya icons')", id);
            }
            Some(id) => {
                println!("  .alya association : {} (not managed by 'alya icons')", id);
            }
            None => {
                println!("  .alya association : (none)");
            }
        }
        match current_default_icon()? {
            Some(icon) => println!("  DefaultIcon       : {}", icon),
            None => println!("  DefaultIcon       : (none)"),
        }
        println!(
            "  icon file         : {} ({})",
            icon_path.display(),
            if icon_path.is_file() {
                "present"
            } else {
                "missing"
            }
        );
        Ok(())
    }

    pub fn install(theme: IconTheme) -> Result<(), String> {
        let bytes = match theme {
            IconTheme::Dark => FILE_ICON_DARK,
            IconTheme::Light => FILE_ICON_LIGHT,
        };
        let dir = icons_dir()?;
        fs::create_dir_all(&dir)
            .map_err(|e| format!("Error: cannot create '{}': {}", dir.display(), e))?;
        let icon_path = dir.join(ICON_FILE_NAME);
        fs::write(&icon_path, bytes)
            .map_err(|e| format!("Error: cannot write '{}': {}", icon_path.display(), e))?;

        match current_progid()? {
            Some(id) if id == PROG_ID => {
                println!("'.alya' is already managed by 'alya icons'; refreshing icon.");
            }
            previous => {
                // Migrate the existing open behavior so double-click keeps
                // working, and remember the ProgID for `uninstall`.
                if let Some(prev) = previous {
                    if let Some(open_cmd) = current_open_command(&prev)? {
                        set_default(&open_command_key(PROG_ID), &open_cmd)?;
                        println!("Migrated open command from '{}'.", prev);
                    } else {
                        println!(
                            "No open command found under '{}'; left unset (Windows will prompt).",
                            prev
                        );
                    }
                    set_named(&prog_key(), PREV_PROPID_VALUE, &prev)?;
                } else {
                    println!("No existing '.alya' association; registering fresh.");
                }
                set_default(&prog_key(), PROG_FRIENDLY)?;
                set_default(&classes_key(EXTENSION), PROG_ID)?;
            }
        }

        set_default(
            &default_icon_key(),
            &format!("\"{}\",0", icon_path.display()),
        )?;
        refresh_icon_cache();
        println!(
            "Registered '{}' with icon '{}'.",
            PROG_ID,
            icon_path.display()
        );
        println!("Close and reopen Explorer windows to see the new icons.");
        Ok(())
    }

    pub fn uninstall() -> Result<(), String> {
        match current_progid()? {
            Some(id) if id == PROG_ID => match query_named(&prog_key(), PREV_PROPID_VALUE)? {
                Some(prev) if !prev.is_empty() => {
                    set_default(&classes_key(EXTENSION), &prev)?;
                    println!("Restored '.alya' association to '{}'.", prev);
                }
                _ => {
                    delete_default(&classes_key(EXTENSION))?;
                    println!("Removed '.alya' association (no previous handler remembered).");
                }
            },
            _ => {
                println!("'.alya' is not managed by 'alya icons'; removing leftovers only.");
            }
        }
        delete_tree(&prog_key())?;

        let icon_path = icons_dir()?.join(ICON_FILE_NAME);
        if icon_path.is_file() {
            fs::remove_file(&icon_path)
                .map_err(|e| format!("Error: cannot remove '{}': {}", icon_path.display(), e))?;
        }
        if let Ok(dir) = icons_dir() {
            // Best effort: drop the directory when we emptied it.
            if dir
                .read_dir()
                .map(|mut d| d.next().is_none())
                .unwrap_or(false)
            {
                let _ = fs::remove_dir(&dir);
            }
        }
        refresh_icon_cache();
        println!("Unregistered '{}'.", PROG_ID);
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::parse_reg_value;

        #[test]
        fn parses_default_value() {
            let out = "HKEY_CURRENT_USER\\Software\\Classes\\.alya\r\n    (Default)    REG_SZ    alya_auto_file\r\n";
            assert_eq!(parse_reg_value(out), Some("alya_auto_file".to_string()));
        }

        #[test]
        fn parses_value_with_spaces() {
            let out = "HKEY_CLASSES_ROOT\\a\\shell\\open\\command\r\n    (Default)    REG_SZ    \"D:\\Program Files\\Microsoft VS Code\\Code.exe\" \"%1\"\r\n";
            assert_eq!(
                parse_reg_value(out),
                Some("\"D:\\Program Files\\Microsoft VS Code\\Code.exe\" \"%1\"".to_string())
            );
        }

        #[test]
        fn missing_key_yields_none() {
            assert_eq!(parse_reg_value(""), None);
            assert_eq!(
                parse_reg_value(
                    "HKEY_CURRENT_USER\\Software\\Classes\\x\r\n    (Default)    REG_SZ\r\n"
                ),
                None
            );
        }

        #[test]
        fn icons_command_variants_are_distinct() {
            use crate::tools::icons::{IconTheme, IconsCommand};
            assert_ne!(
                IconsCommand::Status,
                IconsCommand::Install {
                    theme: IconTheme::Dark
                }
            );
        }
    }
}
