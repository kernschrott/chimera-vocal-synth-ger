//! German text → phoneme segments, by spelling rules and no dictionary (Standard German, as in
//! Duden's pronunciation dictionary; Wiese 1996 for the rules):
//! - Vowel length comes from the spelling: a doubled vowel, "ie" or a vowel + h is long ("Saal",
//!   "Liebe", "Bahn"); a vowel before one consonant and a vowel is long ("Le-ben"), before two or a
//!   doubled consonant short ("Kind", "Mutter", "Zucker"). Before "ch", "sch", "ng", "pf" and "x"
//!   it is short ("Bach", "singen") except in a few listed roots ("Buch", "hoch").
//! - Short e i o u ö ü are lax [ɛ ɪ ɔ ʊ œ y], long ones tense [e i o u ø y]; the length itself is
//!   [`Segment::length`]. ä is [ɛ]. Unstressed e in a word's last syllable and in the prefixes
//!   be- and ge- is schwa ("Liebe", "gesagt"); unstressed final -er is [ɐ] ("Vater").
//! - ch is [x] after a, o, u and au ("Bach", "Buch", "auch"), else [ç] ("ich", "Milch"); -ig at
//!   a word's end is [ɪç] ("König"). b d g are voiceless at a syllable's end (Auslautverhärtung:
//!   "Tag" [taːk], "Hund" [hʊnt]). s before a vowel is [z] ("Sonne"), sp and st start a word as
//!   [ʃp ʃt] ("Stein"). r is uvular [ʁ].
//! - ei/ai [aɪ], au [aʊ], eu/äu [ɔɪ] (the inventory has no [ɔʏ]).
//!
//! Stress falls on the first syllable of the word, after an unstressed prefix (be- ge- er- ver-
//! zer- ent- emp-: "verSTEhen"), on stressed suffixes (-ei, -ieren, -ion, -ität, -ent, -ant, final
//! -ie: "BäckeREI", "stuDIEren", "NaTION"), or as listed ("vielLEICHT", "zuRÜCK").
//!
//! Known gaps, because the rules read spelling, not morphology: a long vowel before an inflection
//! ("Mond", "hört" are read short; "lebt", "sagt" are caught), compounds ("Abendessen" reduces
//! only its last e), loans ("Chef", "Familie"), the glottal stop before a word-initial vowel, and
//! the short ü, which is [y] ([ʏ] isn't in the inventory).

use super::english::{Phrase, is_phrase_punct, split_keeping};
use super::mandarin::MandarinLexicon;
use super::syllables::{FrontEnd, Lang, lyrics_in};
use super::{Intonation, Segment, Syllable};

/// A long vowel lasts this many times what the duration rules give it, a short one this many: the
/// rules come from English, whose tense and lax vowels differ by less than German's long and short
/// ones (and whose /a/ is long).
const LONG_VOWEL: f64 = 1.3;
const SHORT_VOWEL: f64 = 0.8;

fn is_vowel_letter(c: char) -> bool {
    "aeiouäöüyé".contains(c)
}

fn is_letter(c: char) -> bool {
    c.is_ascii_alphabetic() || "äöüßÄÖÜẞéÉ".contains(c)
}

fn de_lower(c: char) -> char {
    match c {
        'ẞ' => 'ß',
        c => c.to_lowercase().next().unwrap_or(c),
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    /// a vowel letter, or two written as one vowel (ie, aa) or a diphthong (ei, au)
    Vowel,
    Consonant,
}

/// One written sound: its letters `a..b`, its sounds, and for a consonant whether it marks the
/// vowel before as short (a doubled letter, ck, tz) or counts as one consonant.
#[derive(Clone, Debug)]
struct Unit {
    a: usize,
    b: usize,
    kind: Kind,
    ipa: Vec<&'static str>,
    /// a vowel: long by spelling (doubled, ie, + h); a consonant: how many consonants it counts as
    /// for the vowel before (2 makes it short)
    weight: usize,
    /// a vowel: written as a diphthong or with "ie", so its quality is fixed
    fixed: bool,
}

fn starts(w: &[char], i: usize, pat: &str) -> bool {
    pat.chars().enumerate().all(|(k, p)| w.get(i + k) == Some(&p))
}

/// Roots whose vowel is long before ch ("Buch", "suchen", "hoch", "Kuchen").
const LONG_BEFORE_CH: &[&str] = &["buch", "büch", "such", "tuch", "tüch", "hoch", "höch", "kuch", "fluch", "sprach", "spräch", "brach", "stach"];

/// Whole words with a short vowel before one consonant at the end ("mit", "das").
#[rustfmt::skip]
const SHORT_WORDS: &[&str] = &[
    "in", "im", "an", "am", "um", "zum", "mit", "es", "des", "das", "was", "bis", "man", "von",
    "hat", "ob", "ab", "hin", "bin", "un", "plus", "bus", "drin", "ran", "ins", "ums",
];

/// Suffixes that start a syllable of their own: the stem's vowel and final consonant are as at a
/// word's end ("Röslein" [ˈʁøːslaɪn] like "Ros", "möglich" [ˈmøːklɪç] like "mög").
const SUFFIXES: &[&str] = &["chen", "lein", "lich", "ling", "los", "bar", "sam", "heit", "keit", "schaft", "tum", "nis", "ung"];

/// Where a syllable-starting suffix starts, if the word ends with one after a stem with a vowel.
fn suffix_start(w: &[char]) -> Option<usize> {
    SUFFIXES.iter().find_map(|sfx| {
        let k = sfx.chars().count();
        let at = w.len().checked_sub(k)?;
        (starts(w, at, sfx) && w[..at].iter().any(|c| is_vowel_letter(*c)) && w.get(at.wrapping_sub(1)).is_some_and(|c| !is_vowel_letter(*c))).then_some(at)
    })
}

/// The letters → written units. `prefix_end`: where an unstressed prefix ends, so "st" and "sp"
/// after it are [ʃt ʃp] as at a word's start ("verstehen"); `suffix`: where a suffix starts, so a
/// consonant before it ends a syllable ("möglich").
fn units(w: &[char], prefix_end: Option<usize>, suffix: Option<usize>) -> Vec<Unit> {
    let mut out: Vec<Unit> = Vec::with_capacity(w.len());
    let n = w.len();
    let mut i = 0;
    let vowel_at = |i: usize| w.get(i).is_some_and(|c| is_vowel_letter(*c));
    while i < n {
        let c = w[i];
        // ---- vowels
        if is_vowel_letter(c) && !(c == 'y' && i == 0 && vowel_at(1)) {
            let two: String = w[i..(i + 2).min(n)].iter().collect();
            let (ipa, len, fixed): (&str, usize, bool) = match two.as_str() {
                "ei" | "ai" | "ey" | "ay" => ("aɪ", 2, true),
                "au" => ("aʊ", 2, true),
                "eu" | "äu" => ("ɔɪ", 2, true),
                "ie" => ("i", 2, true),
                "aa" | "ee" | "oo" => ("", 2, false),
                _ => ("", 1, false),
            };
            let mut b = i + len;
            let mut long = len == 2;
            // a vowel + h is long and the h silent ("Bahn", "gehen"); after a diphthong too
            // ("leihen", "Weihnachten") but not before -heit and -haft ("Freiheit")
            if w.get(b) == Some(&'h') && !(fixed && (starts(w, b, "heit") || starts(w, b, "haft"))) {
                b += 1;
                long = true;
            }
            out.push(Unit { a: i, b, kind: Kind::Vowel, ipa: if ipa.is_empty() { vec![] } else { vec![ipa] }, weight: usize::from(long), fixed });
            i = b;
            continue;
        }
        // ---- consonants
        let initial = i == 0 || Some(i) == prefix_end;
        // the vowel right before (ch after a consonant is [ç]: "durch", "Milch")
        let prev_vowel = out.last().filter(|u| u.kind == Kind::Vowel).map(|u| (w[u.a], w.get(u.a + 1).copied()));
        let back = prev_vowel.is_some_and(|(v, n2)| matches!(v, 'a' | 'o' | 'u') && !(v == 'a' && matches!(n2, Some('i' | 'y'))) || (v == 'a' && n2 == Some('u')));
        // the s of "Hänschen", "bisschen" before the suffix -chen is its own
        let s_chen = c == 's' && suffix == Some(i + 1);
        let (ipa, len, weight): (Vec<&'static str>, usize, usize) = if starts(w, i, "tsch") && suffix != Some(i + 2) {
            (vec!["tʃ"], 4, 2)
        } else if i == 0 && starts(w, i, "dsch") {
            (vec!["dʒ"], 4, 2)
        } else if starts(w, i, "sch") && !s_chen {
            (vec!["ʃ"], 3, 2)
        } else if starts(w, i, "chs") {
            // "sechs", "wachsen": the s is its own unit
            (vec!["k"], 2, 2)
        } else if starts(w, i, "ch") {
            let ipa = if i == 0 {
                // "Chor", "Christ" [k]; "China", "Chemie" [ç]
                if w.get(2).is_some_and(|c| "aourl".contains(*c)) { "k" } else { "ç" }
            } else if back {
                "x"
            } else {
                "ç"
            };
            (vec![ipa], 2, 2)
        } else if starts(w, i, "ck") {
            (vec!["k"], 2, 2)
        } else if starts(w, i, "ph") {
            (vec!["f"], 2, 1)
        } else if starts(w, i, "th") {
            (vec!["t"], 2, 1)
        } else if starts(w, i, "pf") {
            (vec!["pf"], 2, 2)
        } else if starts(w, i, "tz") {
            (vec!["ts"], 2, 2)
        } else if starts(w, i, "qu") {
            (vec!["k", "v"], 2, 1)
        } else if starts(w, i, "ng") {
            (vec!["ŋ"], 2, 2)
        } else if starts(w, i, "nk") {
            (vec!["ŋ"], 1, 1)
        } else if starts(w, i, "ti") && starts(w, i + 2, "on") && i > 0 {
            // "Nation", "Station"
            (vec!["ts"], 1, 1)
        } else if c == 'ß' {
            (vec!["s"], 1, 1)
        } else if w.get(i + 1) == Some(&c) && c != 'h' && suffix != Some(i + 1) {
            // a doubled consonant is one sound that marks a short vowel ("Mutter", "Wasser")
            let ipa = if c == 's' { vec!["s"] } else { consonant(w, i, initial, i + 2, suffix) };
            (ipa, 2, 2)
        } else if initial && (starts(w, i, "st") || starts(w, i, "sp")) {
            (vec!["ʃ"], 1, 1)
        } else if c == 'h' {
            // h before a vowel ("Haus", "Freiheit"); elsewhere it has been read with its vowel
            (if vowel_at(i + 1) { vec!["h"] } else { vec![] }, 1, 0)
        } else {
            (consonant(w, i, initial, i + 1, suffix), 1, 1)
        };
        out.push(Unit { a: i, b: i + len, kind: Kind::Consonant, ipa, weight, fixed: false });
        i += len;
    }
    out
}

/// The sound of a single consonant letter at `i`, whose unit ends at `end`.
fn consonant(w: &[char], i: usize, initial: bool, end: usize, suffix: Option<usize>) -> Vec<&'static str> {
    let next = w.get(end).copied();
    let prev = i.checked_sub(1).map(|p| w[p]);
    // a syllable's end: the word's end, a suffix's start, or a consonant after it that can't start
    // a syllable with it
    let coda = Some(end) == suffix
        || match next {
            None => true,
            Some(n) => !is_vowel_letter(n) && !matches!(n, 'l' | 'r'),
        };
    let devoice = |voiced: &'static str, voiceless: &'static str| if coda { voiceless } else { voiced };
    match w[i] {
        'b' => vec![devoice("b", "p")],
        'd' => vec![devoice("d", "t")],
        'g' => {
            // -ig at a syllable's end: [ɪç] ("König", "Königs", "Ewigkeit")
            if coda && prev == Some('i') && i >= 2 && !is_vowel_letter(w[i - 2]) && w[..i - 1].iter().any(|c| is_vowel_letter(*c)) {
                vec!["ç"]
            } else {
                vec![devoice("g", "k")]
            }
        }
        'c' => {
            if next.is_some_and(|n| "eiäy".contains(n)) {
                vec!["ts"]
            } else {
                vec!["k"]
            }
        }
        'f' | 'v' => vec!["f"],
        'j' => vec!["j"],
        'k' => vec!["k"],
        'l' => vec!["l"],
        'm' => vec!["m"],
        'n' => vec!["n"],
        'p' => vec!["p"],
        'r' => vec!["ʁ"],
        's' => {
            // [z] before a vowel, at a word's start or after a vowel, l, m, n or r ("Sonne",
            // "Rose", "Linse"); else [s] ("Haus", "Erbse", "ist")
            let voiced_before = initial || prev.is_some_and(|p| is_vowel_letter(p) || matches!(p, 'l' | 'm' | 'n' | 'r' | 'h'));
            if next.is_some_and(is_vowel_letter) && voiced_before { vec!["z"] } else { vec!["s"] }
        }
        't' => vec!["t"],
        'w' => vec!["v"],
        'x' => vec!["k", "s"],
        'y' => vec!["j"],
        'z' => vec!["ts"],
        _ => vec![],
    }
}

// ---------------------------------------------------------------- stress

/// Unstressed prefixes ("verstehen", "gesagt", "Beginn").
const PREFIXES: &[&str] = &["ver", "zer", "emp", "ent", "be", "ge", "er"];

/// Stems that start like a prefix but have none: their first syllable is stressed ("gehen",
/// "Erde", "besser"). They match with an inflection from [`ENDINGS`].
#[rustfmt::skip]
const NOT_PREFIXED: &[&str] = &[
    "geh", "geb", "gegen", "gegend", "gelb", "geld", "gelt", "gern", "gestern", "gerste",
    "erd", "erst", "ernst", "ernt", "erb", "erbs", "erker",
    "bet", "bess", "best", "bes", "berg", "beer", "bern",
    "ent", "end", "vers",
];
const ENDINGS: &[&str] = &["", "e", "en", "er", "es", "em", "n", "s", "t", "st", "et", "ens", "ern"];

/// Whole words with their own stress (0 = the first vowel).
#[rustfmt::skip]
const WORDS: &[(&str, usize)] = &[
    ("vielleicht", 1), ("zurück", 1), ("zusammen", 1), ("zuerst", 1), ("allein", 1), ("alleine", 1),
    ("jedoch", 1), ("sofort", 1), ("woher", 1), ("wohin", 1), ("warum", 1), ("darum", 1), ("dabei", 1),
    ("vorbei", 1), ("herein", 1), ("hinein", 1), ("heraus", 1), ("hinaus", 1), ("herum", 1),
    ("daran", 1), ("darauf", 1), ("davon", 1), ("dafür", 1), ("damit", 1), ("dazu", 1), ("hierher", 1),
    ("überall", 2), ("natürlich", 1), ("lebendig", 1), ("forelle", 1), ("musik", 1),
    ("person", 1), ("papier", 1), ("klavier", 1), ("hotel", 1), ("kaffee", 1),
    ("tomate", 1), ("prinzessin", 1), ("januar", 0), ("februar", 0),
    ("august", 1), ("september", 1), ("oktober", 1), ("november", 1), ("dezember", 1),
    ("april", 1), ("juli", 0), ("juni", 0), ("hallo", 1), ("kartoffel", 1),
    ("lavendel", 1), ("holunder", 1), ("wacholder", 1), ("hurra", 1), ("ade", 1), ("adieu", 1),
    ("familie", 1), ("linie", 0), ("ferien", 0), ("italien", 1), ("spanien", 0), ("lilie", 0),
];

/// Suffixes that take the stress: the suffix, which of its vowels is stressed, and the fewest
/// vowels the rest of the word must have.
#[rustfmt::skip]
const STRESSED_SUFFIXES: &[(&str, usize, usize)] = &[
    ("ieren", 0, 0), ("ierend", 0, 0), ("iert", 0, 0), ("ierte", 0, 0), ("ierten", 0, 0), ("ierst", 0, 0), ("iere", 0, 0),
    ("ionen", 1, 1), ("ion", 1, 1), ("itäten", 1, 1), ("ität", 1, 1),
    ("eien", 0, 1), ("ei", 0, 1), ("ie", 0, 1),
    ("enten", 0, 1), ("ent", 0, 1), ("anten", 0, 1), ("ant", 0, 1), ("isten", 0, 1), ("ist", 0, 1),
    ("uren", 0, 1), ("ur", 0, 1),
];

fn vowel_count(w: &[char]) -> usize {
    units(w, None, None).iter().filter(|u| u.kind == Kind::Vowel).count()
}

/// The ordinal of the stressed vowel unit (0 = the first), and the unstressed prefix, if any.
fn stress(word: &str, w: &[char], vowels: usize, suffix: Option<usize>) -> (Option<usize>, Option<&'static str>) {
    if vowels == 0 {
        return (None, None);
    }
    if let Some((_, k)) = WORDS.iter().find(|(x, _)| *x == word) {
        return (Some((*k).min(vowels - 1)), None);
    }
    let not_prefixed = NOT_PREFIXED.iter().any(|x| word.strip_prefix(x).is_some_and(|rest| ENDINGS.contains(&rest)));
    // the prefix's vowel is its own ("Bein", "Beere" have none) and a vowel follows it
    let prefix = PREFIXES.iter().copied().find(|p| {
        let k = p.chars().count();
        // and a vowel follows it before any suffix ("Entchen" is "Ente" + -chen)
        let rest = &w[k.min(w.len())..suffix.unwrap_or(w.len()).max(k.min(w.len()))];
        !not_prefixed && word.starts_with(p) && w.get(k).is_some_and(|c| !is_vowel_letter(*c)) && vowel_count(rest) > 0
    });
    let first = usize::from(prefix.is_some());
    for &(s, at, min_stem) in STRESSED_SUFFIXES {
        let Some(stem) = word.strip_suffix(s) else { continue };
        let stem: Vec<char> = stem.chars().collect();
        // after a consonant ("Nation", "Polizei" has z), except -ei and -ie, which may follow a
        // vowel letter's unit ("Melodie", "Bäckerei")
        if stem.last().is_none_or(|c| is_vowel_letter(*c)) && !matches!(s, "ei" | "eien" | "ie") {
            continue;
        }
        let k = vowel_count(&stem);
        // the stem's own unit must not swallow the suffix's first letter ("frei" is one vowel)
        if k >= min_stem.max(first + usize::from(min_stem > 0)) && k + at < vowels && vowel_count(&w[stem.len()..]) + k == vowels {
            return (Some(k + at), prefix);
        }
    }
    (Some(first.min(vowels - 1)), prefix)
}

// ---------------------------------------------------------------- vowels

/// One sound of a word, with the vowel unit (syllable) it belongs to if it is a vowel, and the unit
/// it is written with.
struct Phone {
    ipa: &'static str,
    syllable: Option<usize>,
    length: Option<f64>,
    unit: usize,
}

struct Parsed {
    typed: Vec<char>,
    units: Vec<Unit>,
    phones: Vec<Phone>,
    stressed: Option<usize>,
}

fn parse(typed: &str) -> Parsed {
    let mut letters: Vec<char> = Vec::with_capacity(typed.len());
    let mut kept: Vec<char> = Vec::with_capacity(typed.len());
    for c in typed.chars().filter(|c| is_letter(*c)) {
        letters.push(match de_lower(c) {
            'é' => 'e',
            c => c,
        });
        kept.push(c);
    }
    let word: String = letters.iter().collect();
    let suffix = suffix_start(&letters);
    let first_pass = units(&letters, None, suffix);
    let vowels = first_pass.iter().filter(|u| u.kind == Kind::Vowel).count();
    let (stressed, prefix) = stress(&word, &letters, vowels, suffix);
    let prefix_end = prefix.map(|p| p.chars().count());
    let mut us = if prefix.is_some() { units(&letters, prefix_end, suffix) } else { first_pass };
    let vowel_units: Vec<usize> = us.iter().enumerate().filter(|(_, u)| u.kind == Kind::Vowel).map(|(i, _)| i).collect();

    let mut phones = Vec::with_capacity(us.len() + 2);
    let mut drop_r = None;
    for (ui, u) in us.iter().enumerate() {
        if u.kind == Kind::Consonant {
            if Some(ui) != drop_r {
                phones.extend(u.ipa.iter().map(|&ipa| Phone { ipa, syllable: None, length: None, unit: ui }));
            }
            continue;
        }
        let k = vowel_units.iter().position(|&x| x == ui).unwrap_or(0);
        let is_stressed = Some(k) == stressed;
        let last = k + 1 == vowel_units.len();
        if u.fixed {
            let length = (u.ipa[0] == "i").then_some(LONG_VOWEL);
            phones.push(Phone { ipa: u.ipa[0], syllable: Some(k), length, unit: ui });
            continue;
        }
        let v = letters[u.a];
        // the consonants up to the next vowel (or the word's end), and how many they count as
        // (a suffix's consonants don't count: "Röslein" is long like "Ros")
        let next_vowel = vowel_units.get(k + 1).copied().unwrap_or(us.len());
        let cut = suffix.filter(|&sx| u.a < sx).map_or(next_vowel, |sx| us.iter().position(|x| x.a >= sx).unwrap_or(us.len()).min(next_vowel));
        let cons = &us[ui + 1..cut];
        let weight: usize = cons.iter().map(|c| c.weight).sum();
        let at_end = next_vowel == us.len() || cut < next_vowel;
        let root_long_ch = cons.first().is_some_and(|c| starts(&letters, c.a, "ch")) && (word == "nach" || LONG_BEFORE_CH.iter().any(|r| {
            // the root's vowel is this one: the root starts one or two letters before it
            (1..=2).any(|o| u.a >= o && starts(&letters, u.a - o, r))
        }));
        // "lebt", "sagt", "trägt", "liebst": b or g + t/st keeps the stem's long vowel
        let inflected = cons.len() >= 2
            && matches!(letters[cons[0].a], 'b' | 'g')
            && cons[0].weight == 1
            && cons[0].ipa != ["ç"]
            && cons[1..].iter().all(|c| matches!(letters[c.a], 's' | 't'))
            && !matches!(word.as_str(), "gibt" | "gibst");
        // long by spelling (doubled, ie, + h), in a listed root or an inflected stem, before no
        // consonant, or before one and a vowel: an open syllable ("Le-ben", "Stra-ße", ß too).
        // A closed last syllable with one consonant is long when stressed ("Tag", "der", "Fuß"),
        // except the listed short words; unstressed ones are short ("König", "Ergebnis")
        let long = u.weight > 0
            || root_long_ch
            || inflected
            || weight == 0
            || (weight == 1 && (!at_end || (is_stressed && !SHORT_WORDS.contains(&word.as_str()))));
        // unstressed -er at the end ("Vater", "immer", "anders", "Kindern"): [ɐ], the r with it
        let r_next = cons.first().is_some_and(|c| letters[c.a] == 'r' && c.b == c.a + 1);
        // schwa: unstressed e after the stress ("Liebe", "Lehrerin"), in be- and ge- ("gesagt"),
        // and before the stressed -ei ("Bäckerei"); before the stress it keeps its quality
        // ("Melodie", "Elefant")
        let erei = stressed == Some(k + 1) && word.ends_with("erei");
        let schwa_context = !is_stressed && (stressed.is_some_and(|st| k > st) || erei || (matches!(prefix, Some("be" | "ge")) && k == 0));
        if v == 'e' && !is_stressed && last && r_next && weight <= 2 && vowel_units.len() > 1 {
            phones.push(Phone { ipa: "ɐ", syllable: Some(k), length: None, unit: ui });
            drop_r = Some(ui + 1);
            continue;
        }
        let ipa = match v {
            'a' => "a",
            'e' if schwa_context && u.weight == 0 => "ə",
            'e' if long => "e",
            'e' => "ɛ",
            'ä' => "ɛ",
            // unstressed i after the stress is short ("heilige", "Könige")
            'i' if long && !stressed.is_some_and(|st| k > st) => "i",
            'i' => "ɪ",
            'o' if long => "o",
            'o' => "ɔ",
            'u' if long => "u",
            'u' => "ʊ",
            'ö' if long => "ø",
            'ö' => "œ",
            _ => "y", // ü, y (short ü would be [ʏ])
        };
        // an unstressed long vowel isn't lengthened ("Melodie"'s e and o)
        let length = if ipa == "ə" || (long && !is_stressed) {
            None
        } else if long {
            Some(LONG_VOWEL)
        } else {
            Some(SHORT_VOWEL)
        };
        phones.push(Phone { ipa, syllable: Some(k), length, unit: ui });
    }
    // the dropped r leaves its unit with no sound
    if let Some(r) = drop_r {
        us[r].ipa.clear();
    }
    Parsed { typed: kept, units: us, phones, stressed }
}

impl Parsed {
    fn segment(&self, p: &Phone) -> Segment {
        Segment {
            ipa: p.ipa.to_string(),
            length: p.length,
            stress: Some(u8::from(p.syllable.is_some() && p.syllable == self.stressed)),
            ..Default::default()
        }
    }
}

/// One word, as typed, to segments with stress.
pub fn word(typed: &str) -> Vec<Segment> {
    let p = parse(typed);
    p.phones.iter().map(|ph| p.segment(ph)).collect()
}

/// One word's sung syllables. Of the consonants between two vowels the last written one opens the
/// next syllable, as German hyphenation splits words ("Lie-be", "Kin-der", "Zu-cker", "Fla-sche");
/// ng stays with the vowel before ("sin-gen", sung [zɪŋ-ən]).
fn sung_syllables(typed: &str) -> Vec<Syllable> {
    let p = parse(typed);
    let vowel_units: Vec<usize> = p.units.iter().enumerate().filter(|(_, u)| u.kind == Kind::Vowel).map(|(i, _)| i).collect();
    if vowel_units.is_empty() {
        return Vec::new();
    }
    // the unit each syllable starts at
    let mut starts: Vec<usize> = vec![0];
    for v in vowel_units.windows(2) {
        let (a, b) = (v[0], v[1]);
        // the consonant units between, ignoring silent ones
        let between: Vec<usize> = (a + 1..b).filter(|&i| !p.units[i].ipa.is_empty()).collect();
        let start = match between.as_slice() {
            [] => b,
            [.., l] if p.units[*l].ipa == ["ŋ"] => l + 1,
            // clusters that start German words start a syllable together: [ʃt ʃp ʃv ʃm ʃn]
            // ("ver-ste-hen", "ge-schwom-men") and a stop or [f] before [l ʁ] ("ge-flo-gen")
            [.., c1, c2] if is_onset(&p.units[*c1].ipa, &p.units[*c2].ipa) => *c1,
            [.., l] => *l,
        };
        starts.push(start.max(a + 1));
    }
    // where a syllable's spelling starts: a silent h read with the vowel before goes with the next
    // syllable, as German splits it ("ge-hen", "ru-hig")
    let letter = |u: usize| {
        p.units.get(u).map_or(p.typed.len(), |x| {
            let h_before = u > 0 && x.kind == Kind::Vowel && p.units[u - 1].kind == Kind::Vowel && p.units[u - 1].b - p.units[u - 1].a > 1 && p.typed[x.a - 1].to_lowercase().eq(['h']);
            x.a - usize::from(h_before)
        })
    };
    let mut out: Vec<Syllable> = starts
        .iter()
        .enumerate()
        .map(|(k, &a)| {
            let b = starts.get(k + 1).map_or(p.typed.len(), |&b| letter(b));
            Syllable { text: p.typed[letter(a).min(b)..b].iter().collect(), ..Default::default() }
        })
        .collect();
    let mut voiced = vec![false; out.len()];
    for ph in &p.phones {
        let k = starts.iter().rposition(|&a| a <= ph.unit).unwrap_or(0);
        // a note sets its own lengths
        let seg = Segment { length: None, ..p.segment(ph) };
        if ph.syllable.is_some() {
            if !voiced[k] {
                out[k].nucleus = seg;
                voiced[k] = true;
            }
        } else if voiced[k] {
            out[k].coda.push(seg);
        } else {
            out[k].onset.push(seg);
        }
    }
    out
}

fn is_onset(a: &[&str], b: &[&str]) -> bool {
    match (a, b) {
        (["ʃ"], ["t" | "p" | "v" | "m" | "n" | "l" | "ʁ"]) => true,
        (["p" | "b" | "t" | "d" | "k" | "g" | "f" | "pf"], ["l" | "ʁ"]) => !matches!((a, b), (["t" | "d"], ["l"])),
        _ => false,
    }
}

/// The German front end for the lyrics router ([`super::syllables::read_lyrics`]). German is
/// written in Latin letters, so the router can't tell it from English by script: the player
/// chooses it ([`Lang`]). Typed hyphens split a word your way, as in English.
pub struct German;

impl FrontEnd for German {
    fn starts(&self, c: char) -> bool {
        is_letter(c)
    }
    fn continues(&self, c: char) -> bool {
        is_letter(c) || c == '-' || c == '\''
    }
    fn word(&self, tok: &str) -> Vec<Syllable> {
        tok.split('-').filter(|p| p.chars().any(is_letter)).flat_map(sung_syllables).collect()
    }
}

/// Lyrics → sung syllables with Latin script read as German ([`Lang::De`]); other scripts are
/// routed as usual, and no dictionary is needed.
pub fn lyrics_to_syllables(text: &str) -> Vec<Syllable> {
    lyrics_in(text, &|_: &str, _: bool| Vec::<Segment>::new(), &MandarinLexicon::new(), Lang::De)
}

/// Question words: a question that starts with one falls like a statement; a yes/no question rises.
const W_WORDS: &[&str] = &["wer", "wen", "wem", "wessen", "was", "wie", "wo", "wann", "warum", "weshalb", "wieso", "weswegen", "woher", "wohin", "womit", "wozu", "welche", "welcher", "welches", "welchen", "welchem", "wieviel"];

/// German text → phrases of segments. Phrase punctuation breaks phrases as in English; a yes/no
/// question rises, a question with a question word falls; numbers are read out ("21" →
/// "einundzwanzig", "3,5" → "drei Komma fünf").
pub fn to_phrases(text: &str) -> Vec<Phrase> {
    let normalized = expand_numbers(&text.replace('’', "'"));
    let mut phrases = Vec::new();
    let mut words: Vec<String> = Vec::new();
    let mut flush = |delim: &str, words: &mut Vec<String>| {
        let mut segs: Vec<Segment> = Vec::new();
        let mut said: Vec<String> = Vec::new();
        for w in words.drain(..) {
            let mut ws = word(&w);
            let Some(last) = ws.last_mut() else { continue };
            last.word_end = Some(true);
            segs.append(&mut ws);
            said.push(w.chars().filter(|c| is_letter(*c)).map(de_lower).collect());
        }
        if segs.is_empty() {
            return;
        }
        let q = delim.contains('?') && !said.first().is_some_and(|w| W_WORDS.contains(&w.as_str()));
        let pause = if delim.contains(['.', '!', '?']) {
            0.35
        } else if !delim.is_empty() {
            0.18
        } else {
            0.3
        };
        phrases.push(Phrase { segments: segs, intonation: if q { Intonation::Question } else { Intonation::Statement }, pause, words: said });
    };
    for part in split_keeping(&normalized, is_phrase_punct) {
        if !part.is_empty() && part.chars().all(is_phrase_punct) {
            flush(&part, &mut words);
        } else {
            words.extend(part.split(|c: char| !is_letter(c)).filter(|w| !w.is_empty()).map(str::to_string));
        }
    }
    flush("", &mut words);
    phrases
}

// ---------------------------------------------------------------- numbers

const ONES: [&str; 20] = [
    "null", "eins", "zwei", "drei", "vier", "fünf", "sechs", "sieben", "acht", "neun", "zehn", "elf", "zwölf", "dreizehn", "vierzehn", "fünfzehn", "sechzehn", "siebzehn", "achtzehn", "neunzehn",
];
const TENS: [&str; 10] = ["", "zehn", "zwanzig", "dreißig", "vierzig", "fünfzig", "sechzig", "siebzig", "achtzig", "neunzig"];

/// 1..999 as one word; "ein" stands for one before "und", "hundert" and "tausend".
fn below_thousand(n: u64, out: &mut String) {
    let (h, r) = (n / 100, n % 100);
    if h > 0 {
        if h > 1 {
            out.push_str(ONES[h as usize]);
        }
        out.push_str("hundert");
    }
    match r {
        0 => {}
        1 if h > 0 => out.push_str("eins"),
        1..=19 => out.push_str(ONES[r as usize]),
        _ => {
            let (t, o) = (r / 10, r % 10);
            if o > 0 {
                out.push_str(if o == 1 { "ein" } else { ONES[o as usize] });
                out.push_str("und");
            }
            out.push_str(TENS[t as usize]);
        }
    }
}

/// Digits → German number words (digit by digit from a billion on).
fn number_words(digits: &str) -> String {
    let d = digits.trim_start_matches('0');
    if d.is_empty() {
        return ONES[0].into();
    }
    let Some(n) = d.parse::<u64>().ok().filter(|n| *n < 1_000_000_000) else {
        return digits.bytes().map(|b| ONES[(b - b'0') as usize]).collect::<Vec<_>>().join(" ");
    };
    let mut out = String::new();
    let (m, t, r) = (n / 1_000_000, n / 1000 % 1000, n % 1000);
    if m > 0 {
        if m == 1 {
            out.push_str("eine Million ");
        } else {
            below_thousand(m, &mut out);
            out.push_str(" Millionen ");
        }
    }
    if t > 0 {
        if t > 1 {
            below_thousand(t, &mut out);
            if out.ends_with("eins") {
                out.truncate(out.len() - 1);
            }
        }
        out.push_str("tausend");
    }
    below_thousand(r, &mut out);
    out.trim().to_string()
}

/// Numbers in text → words. A dot before three digits groups thousands ("1.000"), a comma between
/// digits is the decimal comma ("3,05" → "drei Komma null fünf").
fn expand_numbers(t: &str) -> String {
    let c: Vec<char> = t.chars().collect();
    let digit = |i: usize| c.get(i).is_some_and(char::is_ascii_digit);
    let mut out = String::with_capacity(t.len());
    let mut i = 0;
    while i < c.len() {
        if !digit(i) {
            out.push(c[i]);
            i += 1;
            continue;
        }
        let mut int = String::new();
        loop {
            while digit(i) {
                int.push(c[i]);
                i += 1;
            }
            if c.get(i) == Some(&'.') && digit(i + 1) && digit(i + 2) && digit(i + 3) && !digit(i + 4) {
                i += 1;
                continue;
            }
            break;
        }
        out.push(' ');
        out.push_str(&number_words(&int));
        if c.get(i) == Some(&',') && digit(i + 1) {
            i += 1;
            out.push_str(" Komma");
            while digit(i) {
                out.push(' ');
                out.push_str(ONES[(c[i] as u8 - b'0') as usize]);
                i += 1;
            }
        }
        out.push(' ');
    }
    out
}
