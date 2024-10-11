use std::hash::Hash;

use crate::{ColumnOperations, ColumnOrdering, SelectableTable};

#[derive(Default)]
pub struct AutoReload {
    pub reload_after: Option<u32>,
    pub reload_count: u32,
}
impl AutoReload {
    pub(crate) fn increment_count(&mut self) -> bool {
        self.reload_count += 1;
        if let Some(count) = self.reload_after {
            let reload = self.reload_count >= count;
            if reload {
                self.reload_count = 0;
            }
            reload
        } else {
            false
        }
    }
}

impl<Row, F, Conf> SelectableTable<Row, F, Conf>
where
    Row: Clone + Send + Sync,
    F: Eq
        + Hash
        + Clone
        + Ord
        + Send
        + Sync
        + Default
        + ColumnOperations<Row, F, Conf>
        + ColumnOrdering<Row>,
    Conf: Default,
{
    #[must_use]
    pub const fn auto_reload(mut self, count: u32) -> Self {
        self.auto_reload.reload_after = Some(count);
        self.auto_reload.reload_count = 0;
        self
    }

    pub fn set_auto_reload(&mut self, count: Option<u32>) {
        self.auto_reload.reload_after = count;
        self.auto_reload.reload_count = 0;
    }
}
