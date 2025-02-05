struct Command {
    name: String,
    subs: Vec<Subcommand>,
}

struct Subcommand {
    name: String,
    args: Vec<ParamType>,
}

enum ParamType {
    Int,
    String,
    Regex,
    Color,
    Bool,
}

enum ConfigAction {
    // Single value options
    Chop(bool),
    AltScreen(bool),
    SemanticColor(bool),
    Color(bool),
    Visual(bool),
    MouseScroll(u16),

    // Multi-value options, 0..n
    Filename(PathBuf),
    Search(int, String),
    Filter(int, String),
    // Style(String, PattColor),

    // todo: scroll, SearchEnable, SearchDisable, etc.
}
