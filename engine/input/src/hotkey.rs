//! The hotkey value type, its text form, and the reasons a registration can be
//! refused.
//!
//! Nothing here calls Windows. The platform side is in `registrar`, and the
//! split exists so the validation, the parsing, and the error messages can be
//! tested without a desktop session.

use std::fmt;
use std::str::FromStr;

/// The modifier keys held with the main key.
///
/// A hotkey with none of these set is refused, so the "no modifiers" case is
/// representable on purpose: it has to be, in order to be reported as the
/// specific mistake it is rather than as a generic parse failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Modifiers {
    /// Either Ctrl key.
    pub ctrl: bool,
    /// Either Alt key. On layouts that have an AltGr key, Windows reports AltGr
    /// as Ctrl and Alt together, so a Ctrl+Alt binding fires on characters the
    /// user was only trying to type.
    pub alt: bool,
    /// Either Shift key.
    pub shift: bool,
    /// Either Windows key.
    pub win: bool,
}

impl Modifiers {
    /// No modifiers. Only useful for building up a set or for testing the
    /// refusal.
    pub const NONE: Modifiers = Modifiers {
        ctrl: false,
        alt: false,
        shift: false,
        win: false,
    };

    /// Ctrl and Shift, the pair the default binding uses.
    pub const CTRL_SHIFT: Modifiers = Modifiers {
        ctrl: true,
        alt: false,
        shift: true,
        win: false,
    };

    /// Whether no modifier at all is held.
    pub fn is_empty(self) -> bool {
        self == Modifiers::NONE
    }

    /// How many modifiers are held.
    pub fn count(self) -> u32 {
        u32::from(self.ctrl) + u32::from(self.alt) + u32::from(self.shift) + u32::from(self.win)
    }

    /// Whether this is the AltGr chord as Windows reports it.
    ///
    /// Worth its own name because it is the trap that rules out the otherwise
    /// obvious Ctrl+Alt defaults: on a Nordic or German layout, AltGr is how
    /// the user types braces, brackets, and the at sign.
    pub fn collide_with_altgr(self) -> bool {
        self.ctrl && self.alt
    }
}

/// A non-modifier key, held as its Windows virtual-key code.
///
/// Opaque rather than an enum of every key so that a value cannot be built out
/// of a number that is not a key. Virtual-key codes are used instead of
/// characters because they do not move when the keyboard layout changes, and a
/// default binding that lands on a different physical key in Norway than in the
/// United States is a binding the user did not choose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key(u16);

/// The keys that are neither a letter, a digit, nor a function key, paired with
/// the spelling used in settings and on screen.
const NAMED_KEYS: &[(&str, u16)] = &[
    ("Space", 0x20),
    ("Tab", 0x09),
    ("Backspace", 0x08),
    ("Enter", 0x0D),
    ("Escape", 0x1B),
    ("Insert", 0x2D),
    ("Delete", 0x2E),
    ("Home", 0x24),
    ("End", 0x23),
    ("PageUp", 0x21),
    ("PageDown", 0x22),
    ("Pause", 0x13),
    ("ScrollLock", 0x91),
    ("PrintScreen", 0x2C),
];

impl Key {
    /// F9, the key the default binding uses.
    pub const F9: Key = Key(0x78);

    /// A function key, F1 through F24.
    pub fn function(n: u8) -> Option<Key> {
        (1..=24).contains(&n).then(|| Key(0x6F + u16::from(n)))
    }

    /// A letter key, matched by the printed letter rather than by what the key
    /// types on the current layout.
    pub fn letter(c: char) -> Option<Key> {
        c.is_ascii_alphabetic()
            .then(|| Key(c.to_ascii_uppercase() as u16))
    }

    /// A digit key from the number row, 0 through 9.
    pub fn digit(d: u8) -> Option<Key> {
        (d <= 9).then(|| Key(0x30 + u16::from(d)))
    }

    /// A key looked up by the name it is spelled with in settings.
    pub fn named(name: &str) -> Option<Key> {
        NAMED_KEYS
            .iter()
            .find(|(spelling, _)| spelling.eq_ignore_ascii_case(name))
            .map(|&(_, vk)| Key(vk))
    }

    /// The Windows virtual-key code, for passing to `RegisterHotKey`.
    pub fn virtual_key(self) -> u16 {
        self.0
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            vk @ 0x30..=0x39 => write!(f, "{}", vk - 0x30),
            vk @ 0x41..=0x5A => write!(f, "{}", char::from(vk as u8)),
            vk @ 0x70..=0x87 => write!(f, "F{}", vk - 0x6F),
            vk => match NAMED_KEYS.iter().find(|&&(_, code)| code == vk) {
                Some((spelling, _)) => f.write_str(spelling),
                // Unreachable through the constructors, but a hex code is a
                // more useful thing to show a user filing a bug than a panic.
                None => write!(f, "VK{vk:#04X}"),
            },
        }
    }
}

/// A modifier combination and a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hotkey {
    /// The modifiers held with the key.
    pub modifiers: Modifiers,
    /// The key itself.
    pub key: Key,
}

/// The binding Stasis uses until the user changes it.
///
/// Ctrl+Shift+F9, chosen against a list of things it must not be:
///
/// - Not a Windows binding. Alt+Tab, Alt+Enter, Win+G, Ctrl+Shift+Esc and
///   Shift+F10 are all spoken for by the shell; Ctrl+Shift+F9 is not.
/// - Not another overlay's toggle. Steam takes Shift+Tab, Discord takes Shift
///   and the backtick key, GeForce Experience takes Alt+Z, Adrenalin takes
///   Alt+R, and Game Bar takes Win+G.
/// - Not Ctrl+Alt anything. Windows reports AltGr as Ctrl and Alt together, so
///   on a Nordic or German layout that binding fires while the user is typing
///   a brace.
/// - Not a key whose position moves with the layout. F9 and the two modifiers
///   are virtual keys, so the default is the same physical chord everywhere,
///   which the backtick key that Discord uses is not.
/// - Not a bare or single-modifier function key. Games bind F1 through F8 for
///   pings and loadouts, and F12 is Steam's screenshot key, but a three-key
///   chord on a function key is not something a game binds.
///
/// The collision it does have, stated rather than left to be discovered: Visual
/// Studio and IntelliJ both bind Ctrl+Shift+F9 within their own windows, and a
/// global hotkey takes it from them. That is what rebinding is for.
pub const DEFAULT_HOTKEY: Hotkey = Hotkey {
    modifiers: Modifiers::CTRL_SHIFT,
    key: Key::F9,
};

/// Combinations Windows keeps for itself, which `RegisterHotKey` either refuses
/// or silently never delivers.
///
/// Checked before calling Windows so the user is told which reservation they
/// hit, rather than getting the same "already taken" message that an ordinary
/// conflict produces.
const RESERVED: &[(Modifiers, u16)] = &[
    // Secure attention sequence. Cannot be intercepted by anything.
    (
        Modifiers {
            ctrl: true,
            alt: true,
            shift: false,
            win: false,
        },
        0x2E,
    ),
    // Task Manager.
    (
        Modifiers {
            ctrl: true,
            alt: false,
            shift: true,
            win: false,
        },
        0x1B,
    ),
    // Start menu.
    (
        Modifiers {
            ctrl: true,
            alt: false,
            shift: false,
            win: false,
        },
        0x1B,
    ),
    // Task switcher, and the window cycler beside it.
    (
        Modifiers {
            ctrl: false,
            alt: true,
            shift: false,
            win: false,
        },
        0x09,
    ),
    (
        Modifiers {
            ctrl: false,
            alt: true,
            shift: false,
            win: false,
        },
        0x1B,
    ),
    // Lock workstation.
    (
        Modifiers {
            ctrl: false,
            alt: false,
            shift: false,
            win: true,
        },
        0x4C,
    ),
];

impl Hotkey {
    /// A hotkey from its parts.
    pub fn new(modifiers: Modifiers, key: Key) -> Hotkey {
        Hotkey { modifiers, key }
    }

    /// The reasons this combination can be rejected without asking Windows.
    ///
    /// Run before registering. Both cases produce a worse failure if they are
    /// left to the platform: a modifier-less binding registers successfully and
    /// then eats that key from every application on the desktop, and a reserved
    /// combination fails with an error code that says nothing about why.
    pub fn check(self) -> Result<(), HotkeyError> {
        if self.modifiers.is_empty() {
            return Err(HotkeyError::NoModifier(self));
        }
        if RESERVED
            .iter()
            .any(|&(mods, vk)| mods == self.modifiers && vk == self.key.virtual_key())
        {
            return Err(HotkeyError::Reserved(self));
        }
        Ok(())
    }
}

impl fmt::Display for Hotkey {
    /// Always in the same order, so a binding read out of settings and a
    /// binding shown in an error message are the same string.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (held, name) in [
            (self.modifiers.ctrl, "Ctrl"),
            (self.modifiers.alt, "Alt"),
            (self.modifiers.shift, "Shift"),
            (self.modifiers.win, "Win"),
        ] {
            if held {
                write!(f, "{name}+")?;
            }
        }
        write!(f, "{}", self.key)
    }
}

/// Why a piece of text is not a hotkey.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseHotkeyError {
    /// The text was empty or only separators.
    Empty,
    /// Only modifiers were given, with no key to hold them with.
    NoKey,
    /// A part matched neither a modifier name nor a key name.
    UnknownPart(String),
    /// Two different keys were given, as in `Ctrl+A+B`.
    TwoKeys,
}

impl fmt::Display for ParseHotkeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseHotkeyError::Empty => f.write_str("a hotkey needs at least one key"),
            ParseHotkeyError::NoKey => {
                f.write_str("modifiers on their own are not a hotkey, add a key such as F9")
            }
            ParseHotkeyError::UnknownPart(part) => {
                write!(f, "\"{part}\" is not a key this build recognises")
            }
            ParseHotkeyError::TwoKeys => {
                f.write_str("a hotkey has one key, plus modifiers such as Ctrl and Shift")
            }
        }
    }
}

impl std::error::Error for ParseHotkeyError {}

impl FromStr for Hotkey {
    type Err = ParseHotkeyError;

    /// Reads `Ctrl+Shift+F9` and the variations a settings file collects over
    /// time: any case, any order, and spaces around the separators.
    fn from_str(text: &str) -> Result<Hotkey, ParseHotkeyError> {
        let mut modifiers = Modifiers::NONE;
        let mut key: Option<Key> = None;
        let mut saw_part = false;

        for part in text.split('+').map(str::trim).filter(|p| !p.is_empty()) {
            saw_part = true;
            let held = match part.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => Some(&mut modifiers.ctrl),
                "alt" => Some(&mut modifiers.alt),
                "shift" => Some(&mut modifiers.shift),
                "win" | "super" | "meta" => Some(&mut modifiers.win),
                _ => None,
            };
            if let Some(flag) = held {
                *flag = true;
                continue;
            }

            let parsed =
                parse_key(part).ok_or_else(|| ParseHotkeyError::UnknownPart(part.into()))?;
            if key.is_some_and(|existing| existing != parsed) {
                return Err(ParseHotkeyError::TwoKeys);
            }
            key = Some(parsed);
        }

        if !saw_part {
            return Err(ParseHotkeyError::Empty);
        }
        key.map(|key| Hotkey { modifiers, key })
            .ok_or(ParseHotkeyError::NoKey)
    }
}

fn parse_key(part: &str) -> Option<Key> {
    if let Some(number) = part
        .strip_prefix('F')
        .or_else(|| part.strip_prefix('f'))
        .and_then(|n| n.parse::<u8>().ok())
    {
        return Key::function(number);
    }
    let mut chars = part.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if c.is_ascii_alphabetic() => Key::letter(c),
        (Some(c), None) if c.is_ascii_digit() => Key::digit(c as u8 - b'0'),
        _ => Key::named(part),
    }
}

/// Why a hotkey could not be registered.
///
/// Every variant carries the hotkey, and every message names it. The failure
/// this guards against is a user who presses a combination, sees nothing
/// happen, and has no way to find out that something else on their machine
/// already owns it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HotkeyError {
    /// Another application registered this combination first. Windows does not
    /// say which application, and there is no supported way to ask, so the
    /// message says what is known and no more.
    AlreadyTaken(Hotkey),
    /// Windows keeps this combination for itself.
    Reserved(Hotkey),
    /// The combination has no modifier. Refused before Windows sees it,
    /// because Windows would accept it and the key would then be swallowed
    /// from every application on the desktop.
    NoModifier(Hotkey),
    /// `RegisterHotKey` failed for some other reason, with the code it gave.
    Rejected {
        /// The combination that was asked for.
        hotkey: Hotkey,
        /// The value from `GetLastError`.
        code: u32,
    },
}

impl HotkeyError {
    /// The combination the failure is about.
    pub fn hotkey(&self) -> Hotkey {
        match *self {
            HotkeyError::AlreadyTaken(hotkey)
            | HotkeyError::Reserved(hotkey)
            | HotkeyError::NoModifier(hotkey)
            | HotkeyError::Rejected { hotkey, .. } => hotkey,
        }
    }
}

impl fmt::Display for HotkeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HotkeyError::AlreadyTaken(hotkey) => write!(
                f,
                "{hotkey} is already registered by another application, so Stasis cannot use it. \
                 Pick a different combination."
            ),
            HotkeyError::Reserved(hotkey) => {
                write!(f, "{hotkey} is reserved by Windows and cannot be rebound.")
            }
            HotkeyError::NoModifier(hotkey) => write!(
                f,
                "{hotkey} has no modifier. A hotkey without Ctrl, Alt, Shift or Win would take \
                 that key from every application, so add at least one."
            ),
            HotkeyError::Rejected { hotkey, code } => write!(
                f,
                "Windows refused to register {hotkey} (error {code}). The overlay has no toggle \
                 until this is changed."
            ),
        }
    }
}

impl std::error::Error for HotkeyError {}

#[cfg(test)]
mod tests {
    use super::*;

    // --- the default ------------------------------------------------------

    #[test]
    fn the_default_hotkey_passes_its_own_validation() {
        assert_eq!(DEFAULT_HOTKEY.check(), Ok(()));
    }

    #[test]
    fn the_default_hotkey_is_not_the_altgr_chord() {
        // The reason Ctrl+Alt is not used for the default. This locks it in
        // against a future rethink that reaches for the obvious combination.
        assert!(!DEFAULT_HOTKEY.modifiers.collide_with_altgr());
    }

    #[test]
    fn the_default_hotkey_holds_more_than_one_modifier() {
        // A single modifier plus a function key is the shape games and other
        // overlays actually bind.
        assert!(DEFAULT_HOTKEY.modifiers.count() >= 2);
    }

    #[test]
    fn the_default_hotkey_avoids_the_toggles_other_overlays_already_took() {
        let taken = [
            "Shift+Tab", // Steam
            "Alt+Z",     // GeForce Experience
            "Alt+R",     // Adrenalin
            "Win+G",     // Game Bar
        ];
        for text in taken {
            let other: Hotkey = text.parse().expect("a combination that should parse");
            assert_ne!(DEFAULT_HOTKEY, other, "the default collides with {text}");
        }
    }

    #[test]
    fn the_default_hotkey_reads_back_as_the_string_the_settings_file_holds() {
        assert_eq!(DEFAULT_HOTKEY.to_string(), "Ctrl+Shift+F9");
        assert_eq!("Ctrl+Shift+F9".parse::<Hotkey>(), Ok(DEFAULT_HOTKEY));
    }

    // --- keys -------------------------------------------------------------

    #[test]
    fn function_keys_exist_from_one_to_twenty_four_and_nowhere_else() {
        assert!(Key::function(0).is_none());
        assert!(Key::function(1).is_some());
        assert!(Key::function(24).is_some());
        assert!(Key::function(25).is_none());
    }

    #[test]
    fn function_keys_map_to_the_virtual_key_codes_windows_uses() {
        assert_eq!(Key::function(1).map(Key::virtual_key), Some(0x70));
        assert_eq!(Key::function(9).map(Key::virtual_key), Some(0x78));
        assert_eq!(Key::function(24).map(Key::virtual_key), Some(0x87));
    }

    #[test]
    fn a_letter_is_matched_by_the_printed_letter_in_either_case() {
        // Virtual-key codes for letters are the uppercase ASCII values, so both
        // spellings have to land on the same key rather than two.
        assert_eq!(Key::letter('a'), Key::letter('A'));
        assert_eq!(Key::letter('Z').map(Key::virtual_key), Some(0x5A));
    }

    #[test]
    fn a_character_that_is_not_a_letter_is_not_a_letter_key() {
        for c in ['1', 'å', '+', ' '] {
            assert!(Key::letter(c).is_none(), "{c} was accepted as a letter");
        }
    }

    #[test]
    fn digits_cover_the_number_row_and_stop_there() {
        assert_eq!(Key::digit(0).map(Key::virtual_key), Some(0x30));
        assert_eq!(Key::digit(9).map(Key::virtual_key), Some(0x39));
        assert!(Key::digit(10).is_none());
    }

    #[test]
    fn named_keys_are_matched_without_regard_to_case() {
        assert_eq!(Key::named("pageup"), Key::named("PageUp"));
        assert!(Key::named("Rotate").is_none());
    }

    #[test]
    fn every_key_prints_as_the_name_it_parses_from() {
        // The round trip is what keeps a rebind written to settings readable
        // back out again, so it is checked over the whole set rather than a
        // sample.
        let mut keys: Vec<Key> = Vec::new();
        keys.extend((1..=24).filter_map(Key::function));
        keys.extend(('A'..='Z').filter_map(Key::letter));
        keys.extend((0..=9).filter_map(Key::digit));
        keys.extend(NAMED_KEYS.iter().filter_map(|&(name, _)| Key::named(name)));

        for key in keys {
            let printed = key.to_string();
            assert_eq!(
                parse_key(&printed),
                Some(key),
                "{printed} did not parse back to the key it came from"
            );
        }
    }

    // --- parsing ----------------------------------------------------------

    #[test]
    fn a_hotkey_parses_in_any_case_and_any_order() {
        let canonical: Hotkey = "Ctrl+Shift+F9".parse().expect("canonical form");
        for text in ["ctrl+shift+f9", "SHIFT+CTRL+F9", "Control+Shift+F9"] {
            assert_eq!(
                text.parse::<Hotkey>(),
                Ok(canonical),
                "{text} did not match"
            );
        }
    }

    #[test]
    fn spaces_around_the_separators_are_tolerated() {
        // Settings files get hand-edited, and rejecting a binding over a space
        // would leave the overlay with no toggle for a reason the user cannot
        // see in their own file.
        assert_eq!(" Ctrl + Shift + F9 ".parse::<Hotkey>(), Ok(DEFAULT_HOTKEY));
    }

    #[test]
    fn a_repeated_modifier_is_the_same_as_one_of_them() {
        assert_eq!("Ctrl+Ctrl+Shift+F9".parse::<Hotkey>(), Ok(DEFAULT_HOTKEY));
    }

    #[test]
    fn modifiers_with_no_key_are_refused_with_a_reason_that_names_the_fix() {
        assert_eq!("Ctrl+Shift".parse::<Hotkey>(), Err(ParseHotkeyError::NoKey));
        assert!(ParseHotkeyError::NoKey.to_string().contains("F9"));
    }

    #[test]
    fn an_empty_binding_is_refused_rather_than_read_as_no_hotkey() {
        for text in ["", "   ", "+", " + + "] {
            assert_eq!(
                text.parse::<Hotkey>(),
                Err(ParseHotkeyError::Empty),
                "{text:?} was not refused"
            );
        }
    }

    #[test]
    fn two_different_keys_are_refused_but_the_same_key_twice_is_not() {
        assert_eq!("Ctrl+A+B".parse::<Hotkey>(), Err(ParseHotkeyError::TwoKeys));
        assert!("Ctrl+A+A".parse::<Hotkey>().is_ok());
    }

    #[test]
    fn an_unknown_part_is_named_in_the_error() {
        // The user has to be able to find the typo in their own settings file.
        let error = "Ctrl+Shift+Sparkle".parse::<Hotkey>().expect_err("refused");
        assert_eq!(error, ParseHotkeyError::UnknownPart("Sparkle".into()));
        assert!(error.to_string().contains("Sparkle"));
    }

    #[test]
    fn a_function_key_number_out_of_range_is_reported_as_unknown() {
        let error = "Ctrl+F99".parse::<Hotkey>().expect_err("refused");
        assert_eq!(error, ParseHotkeyError::UnknownPart("F99".into()));
    }

    // --- validation -------------------------------------------------------

    #[test]
    fn a_hotkey_without_a_modifier_is_refused_before_windows_sees_it() {
        let bare = Hotkey::new(Modifiers::NONE, Key::F9);
        assert_eq!(bare.check(), Err(HotkeyError::NoModifier(bare)));
    }

    #[test]
    fn the_combinations_windows_keeps_are_refused_as_reserved() {
        for text in [
            "Ctrl+Alt+Delete",
            "Ctrl+Shift+Escape",
            "Ctrl+Escape",
            "Alt+Tab",
            "Alt+Escape",
            "Win+L",
        ] {
            let hotkey: Hotkey = text.parse().expect("a combination that should parse");
            assert_eq!(
                hotkey.check(),
                Err(HotkeyError::Reserved(hotkey)),
                "{text} was not treated as reserved"
            );
        }
    }

    #[test]
    fn a_reserved_chord_with_an_extra_modifier_is_not_itself_reserved() {
        // Alt+Tab is the switcher; Ctrl+Alt+Shift+Tab is not, and refusing it
        // would be inventing a restriction Windows does not have.
        let hotkey: Hotkey = "Ctrl+Alt+Shift+Tab".parse().expect("parses");
        assert_eq!(hotkey.check(), Ok(()));
    }

    // --- error messages ---------------------------------------------------

    #[test]
    fn every_registration_failure_names_the_combination_it_is_about() {
        // The requirement this crate exists under: a user must never be left
        // wondering which binding failed or whether anything failed at all.
        let hotkey = DEFAULT_HOTKEY;
        let failures = [
            HotkeyError::AlreadyTaken(hotkey),
            HotkeyError::Reserved(hotkey),
            HotkeyError::NoModifier(hotkey),
            HotkeyError::Rejected { hotkey, code: 87 },
        ];
        for failure in failures {
            let message = failure.to_string();
            assert!(
                message.contains("Ctrl+Shift+F9"),
                "{failure:?} produced a message that does not name the hotkey: {message}"
            );
            assert_eq!(failure.hotkey(), hotkey);
        }
    }

    #[test]
    fn the_already_taken_message_says_it_is_taken_rather_than_broken() {
        // "Something went wrong" would send the user looking for a bug in
        // Stasis instead of at the application holding the combination.
        let message = HotkeyError::AlreadyTaken(DEFAULT_HOTKEY).to_string();
        assert!(message.contains("already registered by another application"));
    }

    #[test]
    fn the_rejection_message_carries_the_windows_error_code() {
        let message = HotkeyError::Rejected {
            hotkey: DEFAULT_HOTKEY,
            code: 1400,
        }
        .to_string();
        assert!(message.contains("1400"));
    }

    // --- display ----------------------------------------------------------

    #[test]
    fn modifiers_always_print_in_the_same_order() {
        let all = Modifiers {
            ctrl: true,
            alt: true,
            shift: true,
            win: true,
        };
        assert_eq!(
            Hotkey::new(all, Key::F9).to_string(),
            "Ctrl+Alt+Shift+Win+F9"
        );
    }

    #[test]
    fn a_hotkey_survives_a_round_trip_through_its_own_text_form() {
        let bindings = [
            "Ctrl+Shift+F9",
            "Alt+Shift+Space",
            "Win+Ctrl+PageDown",
            "Ctrl+Alt+Shift+Win+7",
            "Shift+Q",
        ];
        for text in bindings {
            let hotkey: Hotkey = text.parse().expect("a combination that should parse");
            assert_eq!(
                hotkey.to_string().parse::<Hotkey>(),
                Ok(hotkey),
                "{text} did not survive the round trip"
            );
        }
    }
}
