const PORTRAIT_LEVEL_VISUAL: &str = "\
_@_#\n\
_#_#\n\
___#\n\
#_.#\n\
#_.#\n\
#_.#\n\
__##\n\
_$__\n\
_$$_\n\
____";

pub fn portrait_level_ascii() -> String {
    PORTRAIT_LEVEL_VISUAL
        .chars()
        .map(|ch| if ch == '_' { ' ' } else { ch })
        .collect()
}
