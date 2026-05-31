use clap::Subcommand;
use objectiveai_sdk::agent::favorites::{Action, ChangedNotification};

#[derive(Subcommand)]
pub enum Commands {
    /// Get all favorites
    Get,
    /// Add a favorite
    Add {
        #[command(flatten)]
        args: crate::favorite::AddFavorite,
    },
    /// Delete a favorite by name
    Del { name: String },
    /// Edit a favorite
    Edit {
        #[command(flatten)]
        args: crate::favorite::EditFavorite,
    },
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config, handle: &objectiveai_sdk::cli::output::Handle) -> Result<(), crate::error::Error> {
        let (client, mut config) = crate::config::read(cli_config).await?;
        match self {
            Commands::Get => {
                crate::config::emit_value(&config.agents().get_favorites(), handle).await;
                Ok(())
            }
            Commands::Add { args } => {
                let favorite = args.into_favorite()?;
                let name = favorite.get_name().to_string();
                config.agents().add_favorite(favorite);
                crate::config::write(&client, &config, cli_config).await?;
                notify(&mut config, Action::Added, name).await;
                emit_ok(handle).await;
                Ok(())
            }
            Commands::Del { name } => {
                config.agents().del_favorite(&name)?;
                crate::config::write(&client, &config, cli_config).await?;
                notify(&mut config, Action::Removed, name).await;
                emit_ok(handle).await;
                Ok(())
            }
            Commands::Edit { args } => {
                let name = args.name.clone();
                let favorite = config.agents().edit_favorite(&args.name)?;
                args.apply(favorite)?;
                crate::config::write(&client, &config, cli_config).await?;
                notify(&mut config, Action::Edited, name).await;
                emit_ok(handle).await;
                Ok(())
            }
        }
    }
}

async fn notify(
    config: &mut objectiveai_sdk::filesystem::config::Config,
    action: Action,
    name: String,
) {
    let client = crate::api::client::build_viewer_client(config);
    client.send_agents_favorites_changed(
        None,
        None,
        ChangedNotification { action, name },
    );
    client.flush().await;
}

async fn emit_ok(handle: &objectiveai_sdk::cli::output::Handle) {
    objectiveai_sdk::cli::output::Output::Notification(
        objectiveai_sdk::cli::output::Notification {
            agent_id: None,
            value: (objectiveai_sdk::cli::output::OK).into(),
        },
    )
    .emit(handle)
    .await;
}
