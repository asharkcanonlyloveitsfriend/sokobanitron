use crate::grid::Grid;

impl Grid {
    #[inline]
    pub(crate) fn debug_assert_invariants(&self) {
        debug_assert_eq!(self.cells.len(), self.width * self.height);
    }
}
