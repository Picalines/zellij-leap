use zellij_tile::prelude::{PaneId, PaneInfo};

#[derive(Clone)]
pub struct TabIndex(pub usize);

#[derive(Clone)]
pub struct SessionName(pub String);

pub fn pane_id_from_pane(pane_info: &PaneInfo) -> PaneId {
    let id = pane_info.id;
    if pane_info.is_plugin {
        PaneId::Plugin(id)
    } else {
        PaneId::Terminal(id)
    }
}
