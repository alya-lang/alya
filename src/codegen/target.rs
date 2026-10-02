#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Architecture {
    X64,
    ARM64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatingSystem {
    Linux,
    Windows,
    MacOS,
}
