#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BufferId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BufferKind {
    Untitled,
    File,
    ReadOnly,
}
