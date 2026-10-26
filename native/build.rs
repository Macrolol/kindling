//! Records the commit the binary is built from (KINDLING_GIT_SHA, for the run manifest).
fn main() {
    let git = || {
        let out = std::process::Command::new("git").args(["rev-parse", "HEAD"]).output().ok()?;
        out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
    };
    let sha = git().or_else(|| std::env::var("GITHUB_SHA").ok().filter(|s| !s.is_empty()));
    let dirty = std::process::Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=no"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map_or(false, |o| !o.stdout.is_empty());
    let sha = match sha {
        Some(s) if dirty => format!("{s}+dirty"),
        Some(s) => s,
        None => "unknown".into(),
    };
    println!("cargo:rustc-env=KINDLING_GIT_SHA={sha}");
    for p in ["../.git/HEAD", "../.git/index", "../.git/refs/heads"] {
        println!("cargo:rerun-if-changed={p}");
    }
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");
}
