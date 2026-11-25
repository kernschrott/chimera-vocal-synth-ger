# German

Speak and Sing read German once you choose it: **DE**, beside EN and TR in Speak and next to the
lyrics in Sing. It is the same one setting as Turkish, kept on the device with Sing's words.
German is written in Latin letters, so the lyrics router can't tell it from English by script
([language-front-ends.md](language-front-ends.md)); English stays the default. Toned pinyin,
Hangul, Cyrillic and Han characters in the same lyrics still go to their own front ends.

`crates/chimera/src/phonetics/german.rs` reads the spelling by rule, with no dictionary, onto
the existing inventory (no new sounds).

## Writing it

- Type it as you'd write it: ä ö ü ß (ẞ lowers to ß).
- In Sing, a vowel is a note: words split where German hyphenation splits them (Lie-be, Kin-der,
  Zu-cker, ver-ste-hen), except that ng stays with the vowel before (sing-en). `-` alone holds
  the previous vowel onto the next note, and hyphens split a word your way.
- Numbers are read out in Speak ("21" → "einundzwanzig", "3,5" → "drei Komma fünf").

## What it does

| | |
|---|---|
| Vowel length | long when doubled, with h or as ie (Saal, Bahn, Liebe), and before one consonant and a vowel (le-ben); short before two consonants or a doubled one (Kind, Mutter, Zucker). Before ch, sch, ng, pf and x short, except in listed roots (Buch, hoch, suchen). A stressed last syllable with one consonant is long (Tag), except listed short words (mit, das, in). b/g + t keeps the stem long (lebt, sagt). A suffix doesn't shorten its stem (Röslein, möglich, Mädchen) |
| Vowel quality | long e i o u ö ü [e i o u ø y], short [ɛ ɪ ɔ ʊ œ y]; ä [ɛ]. In speech a long vowel lasts 1.3 times what the duration rules give it, a short one 0.8 |
| Schwa and -er | unstressed e after the stress is [ə] (Liebe, Lehrerin), and in be- and ge- (gesagt); unstressed final -er is [ɐ] (Vater, Kindern) |
| ch | [x] right after a, o, u, au (Bach, auch); [ç] elsewhere (ich, euch, durch, Milch, -chen); chs [ks] |
| Final devoicing | b d g are [p t k] at a syllable's end (Hund [hʊnt], Tag [taːk], möglich); -ig is [ɪç] (König), [ɪg] before a vowel (Könige) |
| s, sp, st | [z] before a vowel after a vowel, l m n r or at the start (Sonne, Rose, Linse); [s] elsewhere (Haus, Erbse); sp st start a word or a stem after a prefix as [ʃp ʃt] (Stein, verstehen) |
| r | uvular [ʁ] |
| Diphthongs | ei ai [aɪ], au [aʊ], eu äu [ɔɪ] |
| Stress | the first syllable; after an unstressed prefix (be- ge- er- ver- zer- ent- emp-: verSTEhen) unless the stem only looks prefixed (gehen, Erde, besser); on stressed suffixes (-ei, -ieren, -ion, -ität, -ent, -ant, -ist, -ur, -ie: BäckeREI, stuDIEren, NaTION); or as listed (vielLEICHT, zuRÜCK) |
| Intonation | a yes/no question rises; a question starting with a question word (wer, wie, was, wo, warum, …) falls like a statement |

## Not yet

The rules read spelling, not morphology, so these come out wrong:

- a long vowel before two consonants: Mond, Erde, Pferd, hört are read short;
- compounds: only their last part reduces its e (Abendessen), and sp/st inside one stay [sp st]
  (Bahnsteig);
- loans: Chef, Familie (listed), Universität's v;
- the glottal stop before a word-initial vowel, and vocalised r after a long vowel (der is
  [deːʁ], not [deːɐ̯]);
- short ü, which is [y]: [ʏ] isn't in the inventory, and adding it means a vowel calibration.

A German speaker's ear is still wanted.
