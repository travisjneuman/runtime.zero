pub(crate) struct ToolSpec {
    pub id: &'static str,
    pub display_name: &'static str,
    pub category: &'static str,
    pub names: &'static [&'static str],
    pub version_args: Option<&'static [&'static str]>,
}

#[cfg(windows)]
pub(crate) fn tool_specs() -> &'static [ToolSpec] {
    WINDOWS_TOOL_SPECS
}

#[cfg(not(windows))]
pub(crate) fn tool_specs() -> &'static [ToolSpec] {
    UNIX_TOOL_SPECS
}

#[cfg(windows)]
const WINDOWS_TOOL_SPECS: &[ToolSpec] = &[
    spec(
        "git",
        "Git",
        "version_control",
        &["git.exe"],
        Some(&["--version"]),
    ),
    spec(
        "cargo",
        "Cargo",
        "runtime",
        &["cargo.exe"],
        Some(&["--version"]),
    ),
    spec(
        "rustc",
        "Rust compiler",
        "runtime",
        &["rustc.exe"],
        Some(&["--version"]),
    ),
    spec(
        "node",
        "Node.js",
        "runtime",
        &["node.exe"],
        Some(&["--version"]),
    ),
    spec("npm", "npm", "package_manager", &["npm.cmd"], None),
    spec(
        "python",
        "Python",
        "runtime",
        &["python.exe", "py.exe"],
        Some(&["--version"]),
    ),
    spec(
        "winget",
        "Windows Package Manager",
        "package_manager",
        &["winget.exe"],
        Some(&["--version"]),
    ),
    spec(
        "choco",
        "Chocolatey",
        "package_manager",
        &["choco.exe"],
        Some(&["--version"]),
    ),
    spec("scoop", "Scoop", "package_manager", &["scoop.cmd"], None),
    spec(
        "pwsh",
        "PowerShell",
        "shell",
        &["pwsh.exe"],
        Some(&["--version"]),
    ),
    spec(
        "powershell",
        "Windows PowerShell",
        "shell",
        &["powershell.exe"],
        None,
    ),
];

#[cfg(not(windows))]
const UNIX_TOOL_SPECS: &[ToolSpec] = &[
    spec(
        "git",
        "Git",
        "version_control",
        &["git"],
        Some(&["--version"]),
    ),
    spec(
        "cargo",
        "Cargo",
        "runtime",
        &["cargo"],
        Some(&["--version"]),
    ),
    spec(
        "rustc",
        "Rust compiler",
        "runtime",
        &["rustc"],
        Some(&["--version"]),
    ),
    spec(
        "node",
        "Node.js",
        "runtime",
        &["node"],
        Some(&["--version"]),
    ),
    spec(
        "npm",
        "npm",
        "package_manager",
        &["npm"],
        Some(&["--version"]),
    ),
    spec(
        "python",
        "Python",
        "runtime",
        &["python3", "python"],
        Some(&["--version"]),
    ),
    spec(
        "pwsh",
        "PowerShell",
        "shell",
        &["pwsh"],
        Some(&["--version"]),
    ),
];

const fn spec(
    id: &'static str,
    display_name: &'static str,
    category: &'static str,
    names: &'static [&'static str],
    version_args: Option<&'static [&'static str]>,
) -> ToolSpec {
    ToolSpec {
        id,
        display_name,
        category,
        names,
        version_args,
    }
}
