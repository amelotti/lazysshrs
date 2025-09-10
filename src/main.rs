mod ssh_config;
mod tui;
mod form;
mod config;
mod connectivity;

use ssh_config::SshConfig;
use tui::App;
use config::AppConfig;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app_config = AppConfig::load()?;
    let ssh_config = SshConfig::load_from_workdir(&app_config.get_workdir())?;
    let mut app = App::new(ssh_config, app_config);
    app.run()?;
    Ok(())
}
