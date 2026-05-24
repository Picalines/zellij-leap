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

pub struct Resettable<T> {
    pub current: T,
    initial: T,
}

impl<T: Clone> Resettable<T> {
    pub fn new(initial_value: T) -> Self {
        Self {
            initial: initial_value.clone(),
            current: initial_value,
        }
    }

    pub fn reset(&mut self) {
        self.current = self.initial.clone();
    }
}
