#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SceneCommand<SceneId> {
    None,
    Switch(SceneId),
    Push(SceneId),
    Pop,
    Quit,
}
