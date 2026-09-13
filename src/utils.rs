use std::ops::{Deref, DerefMut};

use zellij_tile::{
    prelude::{PaneId, PaneInfo},
    shim::hide_floating_panes,
};

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

pub fn hide_floating_panes_in_active_tab() {
    _ = hide_floating_panes(None);
}

pub struct Resettable<T> {
    current: T,
    initial: T,
}

impl<T: Clone> Resettable<T> {
    pub fn reset(&mut self) {
        self.current = self.initial.clone();
    }
}

impl<T: Clone> From<T> for Resettable<T> {
    fn from(value: T) -> Self {
        Self {
            current: value.clone(),
            initial: value,
        }
    }
}

impl<T> Deref for Resettable<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.current
    }
}

impl<T> DerefMut for Resettable<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.current
    }
}
