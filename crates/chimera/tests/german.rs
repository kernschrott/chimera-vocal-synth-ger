//! German G2P: vowel length, ch, final devoicing, schwa and -er, stress, numbers, phrases, and the
//! lyrics front end.
use chimera::anatomy::human::BodyMix;
use chimera::phonetics::german::{lyrics_to_syllables, to_phrases, word};
use chimera::phonetics::inventory::phoneme;
use chimera::phonetics::mandarin::MandarinLexicon;
use chimera::phonetics::planner::{PhraseInput, PlanOptions, is_vowel, plan_phrases};
use chimera::phonetics::sing::{SongOptions, SungNote, plan_song};
use chimera::phonetics::syllables::{Lang, lyrics_in};
use chimera::phonetics::{Intonation, Segment, Syllable};

/// Segments as IPA, a stress mark before the stressed vowel, ː after a lengthened one and a space
/// after each word.
fn shown(segs: &[Segment]) -> String {
    segs.iter()
        .map(|s| {
            let long = s.length.is_some_and(|l| l > 1.0);
            format!("{}{}{}{}", if s.stress == Some(1) { "ˈ" } else { "" }, s.ipa, if long { "ː" } else { "" }, if s.word_end == Some(true) { " " } else { "" })
        })
        .collect::<String>()
        .trim_end()
        .to_string()
}
fn de(text: &str) -> String {
    to_phrases(text).iter().map(|p| shown(&p.segments)).collect::<Vec<_>>().join(" | ")
}
fn w(typed: &str) -> String {
    shown(&word(typed))
}

#[test]
fn vowel_length_comes_from_the_spelling() {
    // a vowel + h, a doubled vowel and ie are long
    assert_eq!(w("Bahn"), "bˈaːn");
    assert_eq!(w("Meer"), "mˈeːʁ");
    assert_eq!(w("Liebe"), "lˈiːbə");
    // one consonant before a vowel: an open syllable, long; two or a doubled one: short
    assert_eq!(w("leben"), "lˈeːbən");
    assert_eq!(w("Kind"), "kˈɪnt");
    assert_eq!(w("Mutter"), "mˈʊtɐ");
    assert_eq!(w("Zucker"), "tsˈʊkɐ");
    // a stressed last syllable with one consonant: long, except the listed short words
    assert_eq!(w("Tag"), "tˈaːk");
    assert_eq!(w("mit"), "mˈɪt");
    // ß after a long vowel, ss after a short one
    assert_eq!(w("Straße"), "ʃtʁˈaːsə");
    assert_eq!(w("muss"), "mˈʊs");
    // b or g + t keeps the stem's long vowel ("lebt"), but not in "gibt"
    assert_eq!(w("lebt"), "lˈeːpt");
    assert_eq!(w("gibt"), "gˈɪpt");
    // a suffix doesn't shorten the stem ("Röslein" like "Ros")
    assert_eq!(w("Röslein"), "ʁˈøːslaɪn");
    assert_eq!(w("möglich"), "mˈøːklɪç");
}

#[test]
fn ch_is_ach_or_ich_by_the_vowel_before() {
    assert_eq!(w("Bach"), "bˈax");
    assert_eq!(w("auch"), "ˈaʊx");
    assert_eq!(w("ich"), "ˈɪç");
    assert_eq!(w("euch"), "ˈɔɪç");
    // after a consonant [ç]
    assert_eq!(w("durch"), "dˈʊʁç");
    assert_eq!(w("Milch"), "mˈɪlç");
    // a few roots are long before ch
    assert_eq!(w("Buch"), "bˈuːx");
    assert_eq!(w("Nacht"), "nˈaxt");
    // chs is [ks]; -chen is [çən], its s before it its own ("Hänschen")
    assert_eq!(w("sechs"), "zˈɛks");
    assert_eq!(w("Hänschen"), "hˈɛnsçən");
}

#[test]
fn final_devoicing_and_ig() {
    assert_eq!(w("Hund"), "hˈʊnt");
    assert_eq!(w("Tage"), "tˈaːgə");
    assert_eq!(w("König"), "kˈøːnɪç");
    assert_eq!(w("Könige"), "kˈøːnɪgə");
    // ei + g is no -ig
    assert_eq!(w("Teig"), "tˈaɪk");
}

#[test]
fn s_sp_st() {
    assert_eq!(w("Sonne"), "zˈɔnə");
    assert_eq!(w("Haus"), "hˈaʊs");
    assert_eq!(w("Stein"), "ʃtˈaɪn");
    assert_eq!(w("Spiel"), "ʃpˈiːl");
    // after an unstressed prefix as at the start
    assert_eq!(w("verstehen"), "fɛʁʃtˈeːən");
    // inside a word st is [st]
    assert_eq!(w("Fenster"), "fˈɛnstɐ");
}

#[test]
fn schwa_and_final_er() {
    assert_eq!(w("Vater"), "fˈaːtɐ");
    assert_eq!(w("Kindern"), "kˈɪndɐn");
    assert_eq!(w("gesagt"), "gəzˈaːkt");
    assert_eq!(w("Lehrerin"), "lˈeːʁəʁɪn");
    assert_eq!(w("Bäckerei"), "bɛkəʁˈaɪ");
    // before the stress e keeps its quality
    assert_eq!(w("Melodie"), "melodˈiː");
}

#[test]
fn stress_is_initial_after_prefixes_and_on_stressed_suffixes() {
    assert_eq!(w("Abend"), "ˈaːbənt");
    assert_eq!(w("Beginn"), "bəgˈɪn");
    assert_eq!(w("bestimmt"), "bəʃtˈɪmt");
    // stems that only look prefixed
    assert_eq!(w("gehen"), "gˈeːən");
    assert_eq!(w("besser"), "bˈɛsɐ");
    assert_eq!(w("Entchen"), "ˈɛntçən");
    // stressed suffixes
    assert_eq!(w("Nation"), "natsiˈoːn");
    assert_eq!(w("studieren"), "ʃtudˈiːʁən");
    assert_eq!(w("Student"), "ʃtudˈɛnt");
    // listed words
    assert_eq!(w("vielleicht"), "fiːlˈaɪçt");
}

#[test]
fn numbers_are_read_out() {
    assert_eq!(de("21"), de("einundzwanzig"));
    assert_eq!(de("101"), de("hunderteins"));
    assert_eq!(de("1.000"), de("tausend"));
    assert_eq!(de("3,5"), de("drei Komma fünf"));
}

#[test]
fn a_yes_no_question_rises_and_a_w_question_falls() {
    let ph = to_phrases("Wie geht es dir? Kommst du morgen? Ja.");
    assert_eq!(ph.len(), 3);
    assert_eq!(ph[0].intonation, Intonation::Statement);
    assert_eq!(ph[1].intonation, Intonation::Question);
    assert_eq!(ph[2].intonation, Intonation::Statement);
    assert_eq!(ph[1].words, ["kommst", "du", "morgen"]);
    assert_eq!(ph[1].segments.iter().filter(|s| s.word_end == Some(true)).count(), 3);
}

#[test]
fn every_sound_is_one_the_voice_can_make() {
    let text = "Zwölf Boxkämpfer jagen Viktor quer über den großen Sylter Deich. Fischers Fritz fischt frische Fische; Hänschen klein ging allein.";
    let body = BodyMix::MALE;
    let phrases = to_phrases(text);
    for s in phrases.iter().flat_map(|p| &p.segments) {
        assert!(is_vowel(&s.ipa, body) || phoneme(&s.ipa, body).is_some(), "unknown sound {}", s.ipa);
    }
    let inputs: Vec<PhraseInput> = phrases.iter().map(|p| PhraseInput { segments: p.segments.clone(), intonation: Some(p.intonation), pause: Some(p.pause) }).collect();
    let plan = plan_phrases(&inputs, &PlanOptions { body, ..Default::default() });
    assert!(!plan.gestures.is_empty());
}

#[test]
fn a_long_vowel_lasts_longer_than_a_short_one() {
    let a = |text: &str| {
        let ph = to_phrases(text);
        let inputs: Vec<PhraseInput> = ph.iter().map(|p| PhraseInput { segments: p.segments.clone(), intonation: Some(p.intonation), pause: Some(p.pause) }).collect();
        let plan = plan_phrases(&inputs, &PlanOptions { body: BodyMix::MALE, ..Default::default() });
        plan.segments.iter().find(|s| s.ipa == "a").map(|s| s.t1 - s.t0).unwrap()
    };
    // "Staat" and "Stadt": the same consonants, a long and a short a
    assert!(a("Staat") > 1.4 * a("Stadt"), "{} {}", a("Staat"), a("Stadt"));
}

// ---------------------------------------------------------------- singing

fn sung(text: &str) -> Vec<String> {
    lyrics_to_syllables(text).iter().map(|s| s.text.clone()).collect()
}
fn parts(s: &Syllable) -> String {
    let ipa = |v: &[Segment]| v.iter().map(|x| x.ipa.as_str()).collect::<String>();
    format!("{}.{}.{}", ipa(&s.onset), s.nucleus.ipa, ipa(&s.coda))
}

#[test]
fn lyrics_split_where_german_spelling_does() {
    assert_eq!(sung("Alle meine Entchen"), ["A", "lle", "mei", "ne", "Ent", "chen"]);
    assert_eq!(sung("Kinder Zucker Flasche gehen ruhig"), ["Kin", "der", "Zu", "cker", "Fla", "sche", "ge", "hen", "ru", "hig"]);
    // [ʃt] and a stop + l or r open a syllable together
    assert_eq!(sung("verstehen geflogen"), ["ver", "ste", "hen", "ge", "flo", "gen"]);
    let p: Vec<String> = lyrics_to_syllables("singen Vater").iter().map(parts).collect();
    // ng stays with the vowel before; final -er is one note's [ɐ]
    assert_eq!(p, ["z.ɪ.ŋ", ".ə.n", "f.a.", "t.ɐ."]);
    // sung lengths come from the notes
    assert!(lyrics_to_syllables("Bahn Straße").iter().all(|s| s.nucleus.length.is_none()));
}

#[test]
fn held_notes_and_phrases_in_lyrics() {
    let syl = lyrics_to_syllables("Lie- - be, Herz");
    assert_eq!(syl.iter().map(|s| s.text.as_str()).collect::<Vec<_>>(), ["Lie", "~", "be", "Herz"]);
    assert_eq!(syl[1].melisma, Some(true));
    assert!(syl[2].word_end && syl[2].phrase_end);
}

#[test]
fn latin_script_is_german_only_when_chosen() {
    let no_dict = |_: &str, _: bool| Vec::<Segment>::new();
    let text = |lang| lyrics_in("schön grüß", &no_dict, &MandarinLexicon::new(), lang).into_iter().map(|s| s.text).collect::<Vec<_>>();
    assert_eq!(text(Lang::De), ["schön", "grüß"]);
    // English claims only ASCII letters, as before: ö, ü and ß are skipped
    assert!(text(Lang::En).iter().all(|t| t.is_ascii()));
    // other scripts still go to their own front ends
    let mixed = lyrics_to_syllables("Liebe 빵야 дом");
    assert_eq!(mixed.iter().map(|s| s.text.as_str()).collect::<Vec<_>>(), ["Lie", "be", "빵", "야", "дом"]);
}

#[test]
fn german_lyrics_plan_as_a_song() {
    let syl = lyrics_to_syllables("Stille Nacht, heilige Nacht.");
    let notes: Vec<SungNote> = syl.iter().enumerate().map(|(i, s)| SungNote { midi: 60.0 + i as f64, beats: 1.0, syl: s.clone() }).collect();
    let bpm = 90.0;
    let plan = plan_song(&notes, &SongOptions { body: BodyMix::FEMALE, bpm, t0: 0.35, vibrato: 35.0 });
    assert!(plan.end > 0.35 + notes.len() as f64 * 60.0 / bpm - 0.2, "{}", plan.end);
    assert!(plan.segments.iter().any(|s| s.ipa == "x"));
}
