use anyhow::Result;

use super::*;

pub fn import_mod(core: &Manager, path: PathBuf) -> Result<Message> {
    let project = Project::from_mod(core, &path)?;
    Ok(Message::OpenProject(project))
}

pub fn open_resource(core: &Manager, root: PathBuf, path: PathBuf) -> Result<Message> {
    let file = root.join(canonicalize(&path).as_str());
    let data = fs::read(file)?;
    let resource: ResourceData =
        if data.len() > 10 && &data[..9] == b"Mergeable" || &data[..4] == b"Sarc" {
            let res: ResourceData = ron::from_str(std::str::from_utf8(&data)?)?;
            match core
                .settings()
                .dump()
                .context("No dump for current mode")?
                .get_data(&path)
            {
                Ok(ref_res) => {
                    match (&*ref_res, &res) {
                        (ResourceData::Mergeable(ref_res), ResourceData::Mergeable(res)) => {
                            ResourceData::Mergeable(ref_res.merge(res))
                        }
                        (ResourceData::Sarc(ref_res), ResourceData::Sarc(res)) => {
                            ResourceData::Sarc(ref_res.merge(res))
                        }
                        _ => res,
                    }
                }
                Err(_) => res,
            }
        } else {
            ResourceData::Binary(data)
        };
    Ok(Message::LoadResource(path, resource))
}
