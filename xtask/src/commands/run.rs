use tracel_xtask::prelude::*;

use super::assets;

const APP_PACKAGE: &str = "win11-borderless-gaming-desktop";

#[macros::declare_command_args(None, None)]
pub struct RunCmdArgs {}

pub fn handle_command(_args: RunCmdArgs) -> anyhow::Result<()> {
    assets::generate()?;

    let status = std::process::Command::new("cargo")
        .args(cargo_run_args())
        .status()
        .map_err(|error| anyhow::anyhow!("failed to start Cargo: {error}"))?;

    if !status.success() {
        anyhow::bail!("Cargo could not build or run the release app ({status})");
    }

    Ok(())
}

fn cargo_run_args() -> Vec<&'static str> {
    vec![
        "run",
        "--release",
        "--package",
        APP_PACKAGE,
        "--bin",
        APP_PACKAGE,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_run_uses_the_release_binary() {
        assert_eq!(
            cargo_run_args(),
            [
                "run",
                "--release",
                "--package",
                APP_PACKAGE,
                "--bin",
                APP_PACKAGE,
            ]
        );
    }
}
