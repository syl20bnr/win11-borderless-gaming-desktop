use tracel_xtask::prelude::*;

use super::assets;

#[macros::declare_command_args(None, None)]
pub struct IconsCmdArgs {}

pub fn handle_command(_args: IconsCmdArgs) -> anyhow::Result<()> {
    assets::generate()
}
