use zellij_tile::prelude::{PaneId, PaneInfo};

pub struct TabIndex(pub usize);

pub fn pane_id_from_pane(pane_info: &PaneInfo) -> PaneId {
    let id = pane_info.id;
    if pane_info.is_plugin {
        PaneId::Plugin(id)
    } else {
        PaneId::Terminal(id)
    }
}
