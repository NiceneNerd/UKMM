use anyhow::Result;

use super::*;

pub fn import_mod(core: &Manager, path: PathBuf) -> Result<Message> {
    let project = Project::from_mod(core, &path)?;
    Ok(Message::OpenProject(project))
}
