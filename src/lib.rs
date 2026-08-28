mod finders;
mod patterns;
mod report;
mod text;

pub use patterns::{PATTERNS, Pattern, WIKI_GROUP, pattern};
pub use report::{
    CONTEXT_WORDS, Match, Region, Report, Window, analyze, build_regions, build_windows,
    collect_matches, count_words, sentence_bounds, snippet,
};

pub const EXAMPLE: &str = include_str!("example.txt");

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn all() -> HashSet<&'static str> {
        PATTERNS.iter().map(|p| p.id).collect()
    }

    fn only(id: &'static str) -> HashSet<&'static str> {
        HashSet::from([id])
    }

    #[test]
    fn pattern_cases() {
        let cases: &[(&str, &str, usize, Option<&[usize]>)] = &[
            (
                "no-chain",
                "No sign-ups, no downloads, no hassle \u{2014} just paste and go.",
                1,
                Some(&[3]),
            ),
            (
                "no-chain",
                "The plan has no hidden fees and no long-term contracts.",
                1,
                Some(&[2]),
            ),
            (
                "no-chain",
                "No fluff, no filler, no jargon, no corporate buzzwords.",
                1,
                Some(&[4]),
            ),
            (
                "no-chain",
                "There is no catch here, honestly.",
                0,
                Some(&[]),
            ),
            (
                "no-chain",
                "It ships with no bells and whistles, no fluff.",
                1,
                Some(&[2]),
            ),
            ("no-chain", "No, no, I insist.", 0, Some(&[])),
            ("no-chain", "no no no", 0, Some(&[])),
            (
                "no-chain",
                "with no list patterns at all, so nothing lights up.",
                0,
                Some(&[]),
            ),
            (
                "no-chain",
                "NO FEES, NO CONTRACTS, NO SURPRISES",
                1,
                Some(&[3]),
            ),
            ("no-chain", "no fluff; no filler", 1, Some(&[2])),
            (
                "no-chain",
                "no time, no money, no way to say no thanks",
                1,
                Some(&[3]),
            ),
            ("no-chain", "no-code, no-fuss setup", 1, Some(&[2])),
            ("no-chain", "I know nothing, notice nothing.", 0, Some(&[])),
            (
                "no-chain",
                "No fluff, no filler.\nNo ads here.",
                1,
                Some(&[2]),
            ),
            ("whole", "That's the whole point.", 1, None),
            ("whole", "This is the whole game, really.", 1, None),
            ("whole", "That was the whole pitch.", 1, None),
            ("whole", "The whole team showed up.", 0, None),
            (
                "did-not-chain",
                "Did not flinch, did not blink, did not apologize.",
                1,
                Some(&[3]),
            ),
            (
                "did-not-chain",
                "He didn't call and didn't write.",
                1,
                Some(&[2]),
            ),
            ("did-not-chain", "She did not go.", 0, Some(&[])),
            (
                "did-not-chain",
                "Did not know why, did not care.",
                1,
                Some(&[2]),
            ),
            (
                "dont-verb-it",
                "Don't call it a comeback. Call it a return.",
                1,
                None,
            ),
            (
                "dont-verb-it",
                "Do not think of it as a burden. Think of it as fuel.",
                1,
                None,
            ),
            ("dont-verb-it", "Don't fear it. Name it.", 0, None),
            (
                "dont-verb-it",
                "Don\u{2019}t call it \"luck.\" Call it preparation.",
                1,
                None,
            ),
            (
                "dont-verb-it",
                "Don't just read it \u{2014} read it aloud.",
                1,
                None,
            ),
            ("dont-verb-it", "Don't overthink it.", 0, None),
            ("sit-with", "Sit with that for a moment.", 1, None),
            ("sit-with", "Just sit with it.", 1, None),
            ("sit-with", "She was sitting with the discomfort.", 1, None),
            ("sit-with", "Come sit with us at lunch.", 0, None),
            ("already-know", "You already know the answer.", 1, None),
            ("already-know", "Deep down, you already know.", 1, None),
            (
                "already-know",
                "If you already know Python, skip ahead.",
                0,
                None,
            ),
            ("already-know", "You already know what to do.", 1, None),
            ("already-know", "Part of you already knows it.", 1, None),
            ("is-the-entire", "Consistency is the entire game.", 1, None),
            (
                "is-the-entire",
                "That's the entire business model.",
                1,
                None,
            ),
            ("is-the-entire", "He toured the entire factory.", 0, None),
            (
                "the-entire-is",
                "The entire point is that nobody reads.",
                1,
                None,
            ),
            (
                "the-entire-is",
                "The entire business model is built on churn.",
                1,
                None,
            ),
            (
                "the-entire-is",
                "The entire point of the exercise is repetition.",
                1,
                None,
            ),
            ("the-entire-is", "He ate the entire pizza.", 0, None),
            ("the-entire-is", "The entire team was exhausted.", 1, None),
            (
                "the-entire-is",
                "The entire history of the modern industrial world economy is complex.",
                0,
                None,
            ),
            (
                "is-real",
                "The improvement is real, and it's not subtle.",
                1,
                None,
            ),
            (
                "is-real",
                "This is the real work, and it never ends.",
                1,
                None,
            ),
            ("is-real", "The demand is real and growing.", 1, None),
            (
                "is-real",
                "He is a real estate agent and it shows.",
                0,
                None,
            ),
            ("is-real", "Is it real? And does it matter?", 0, None),
            ("is-real", "The painting is real, but stolen.", 0, None),
            (
                "punchline",
                "The punchline is that nobody laughed.",
                1,
                None,
            ),
            ("punchline", "The punchline: nothing changed.", 1, None),
            ("punchline", "And the punchline? You knew.", 1, None),
            ("punchline", "He forgot the punchline entirely.", 0, None),
            (
                "worth-naming",
                "That loss is real and it's worth naming.",
                1,
                None,
            ),
            (
                "worth-naming",
                "It\u{2019}s worth naming that this hurts.",
                1,
                None,
            ),
            ("worth-naming", "The grief here is worth naming.", 1, None),
            (
                "worth-naming",
                "That anger feels worth naming out loud.",
                1,
                None,
            ),
            (
                "worth-naming",
                "Worth naming: nobody asked for this.",
                1,
                None,
            ),
            ("worth-naming", "It's not worth naming names here.", 0, None),
            (
                "worth-naming",
                "They spent the meeting naming the new mascot.",
                0,
                None,
            ),
            (
                "worth-naming",
                "The naming convention is worth documenting.",
                0,
                None,
            ),
            ("not-nothing", "That's not nothing.", 1, None),
            (
                "not-nothing",
                "Ten sign-ups in a week \u{2014} that is not nothing.",
                1,
                None,
            ),
            (
                "not-nothing",
                "It's not nothing, even if it's not everything.",
                1,
                None,
            ),
            (
                "not-nothing",
                "The launch drew a small crowd, which was not nothing.",
                1,
                None,
            ),
            (
                "not-nothing",
                "She insisted that nothing was wrong.",
                0,
                None,
            ),
            ("not-nothing", "There is nothing left to say.", 0, None),
            ("is-the-whole", "Distribution is the whole game.", 1, None),
            (
                "is-the-whole",
                "Here's the whole pitch in one slide.",
                1,
                None,
            ),
            (
                "is-the-whole",
                "That was the whole point of the meeting.",
                1,
                None,
            ),
            ("is-the-whole", "The whole team showed up.", 0, None),
            (
                "echo-triad",
                "A shopping cart is an object in the system. A chat room is an object in the system.",
                1,
                Some(&[2]),
            ),
            (
                "echo-triad",
                "The parser is a state machine. The renderer is a state machine. The scheduler is a state machine.",
                1,
                Some(&[3]),
            ),
            (
                "echo-triad",
                "The parser is fast today. The renderer is fast today.",
                0,
                Some(&[]),
            ),
            (
                "echo-triad",
                "The parser is fast. The tests are slow.",
                0,
                Some(&[]),
            ),
            (
                "performative-honesty",
                "I won't pretend the migration was painless.",
                1,
                None,
            ),
            (
                "performative-honesty",
                "Let's be honest: nobody reads the docs.",
                1,
                None,
            ),
            (
                "performative-honesty",
                "To be clear, the API is unchanged.",
                1,
                None,
            ),
            ("performative-honesty", "Honestly, it was fine.", 1, None),
            ("performative-honesty", "She answered honestly.", 0, None),
            ("performative-honesty", "Look at the diagram.", 0, None),
            (
                "thats-the-part",
                "That's the part a counter can't reach.",
                1,
                None,
            ),
            (
                "thats-the-part",
                "The part that makes me trust the rest is the errata.",
                1,
                None,
            ),
            (
                "thats-the-part",
                "My favorite part of the demo was the undo.",
                1,
                None,
            ),
            (
                "thats-the-part",
                "He played the part of the villain.",
                0,
                None,
            ),
            (
                "the-only-i-trust",
                "It\u{2019}s the only marketing I trust.",
                1,
                None,
            ),
            (
                "the-only-i-trust",
                "The only benchmark that matters is retention.",
                1,
                None,
            ),
            (
                "the-only-i-trust",
                "The only thing it needs is a cache.",
                1,
                None,
            ),
            (
                "the-only-i-trust",
                "She was the only engineer on call.",
                0,
                None,
            ),
            (
                "take-my-word",
                "You don't have to take my word for it.",
                1,
                None,
            ),
            (
                "take-my-word",
                "Don't take my word for any of this.",
                1,
                None,
            ),
            ("take-my-word", "He kept his word.", 0, None),
            ("turns-out", "Turns out the cache was never warm.", 1, None),
            ("turns-out", "It turns out that nobody tested it.", 1, None),
            ("turns-out", "She turns out solid work every week.", 0, None),
            (
                "fits-in-your-head",
                "The design is small enough to hold in your head.",
                1,
                None,
            ),
            (
                "fits-in-your-head",
                "It ships with sane defaults and zero config.",
                2,
                None,
            ),
            (
                "fits-in-your-head",
                "Install it and it just works.",
                1,
                None,
            ),
            (
                "fits-in-your-head",
                "We choose boring technology on purpose.",
                0,
                None,
            ),
            ("fits-in-your-head", "The helmet fits your head.", 0, None),
            (
                "stacked-questions",
                "Do I know how it works? Where it breaks? Which corners it cut?",
                1,
                Some(&[3]),
            ),
            (
                "stacked-questions",
                "Was it worth it? Would I do it again?",
                1,
                Some(&[2]),
            ),
            (
                "stacked-questions",
                "Did it work? Yes, and then some.",
                0,
                Some(&[]),
            ),
            ("stacked-questions", "What changed?", 0, Some(&[])),
            (
                "sentence-anaphora",
                "Maybe nobody needed it. Maybe the timing was off. Maybe both.",
                1,
                Some(&[3]),
            ),
            (
                "sentence-anaphora",
                "Maybe nobody needed it. Maybe the timing was off.",
                0,
                Some(&[]),
            ),
            (
                "sentence-anaphora",
                "The parser is small. The renderer is small. The scheduler is small.",
                0,
                Some(&[]),
            ),
            (
                "sentence-anaphora",
                "Everything changed. Everything slowed down. Everything cost more.",
                1,
                Some(&[3]),
            ),
            (
                "colon-triple",
                "The fix needs three things: separate ports, separate processes, and separate state.",
                1,
                None,
            ),
            (
                "colon-triple",
                "Each service gets its own everything: ports, processes, local state.",
                1,
                None,
            ),
            (
                "colon-triple",
                "The recipe calls for flour, butter, and sugar.",
                0,
                None,
            ),
            ("colon-triple", "Note: the flag is off by default.", 0, None),
            (
                "heres-the-twist",
                "Here's the twist: nobody clicked it.",
                1,
                None,
            ),
            (
                "heres-the-twist",
                "Here is the thing. The demo was fake.",
                1,
                None,
            ),
            (
                "heres-the-twist",
                "Here's a surprising result: it got faster.",
                1,
                None,
            ),
            ("heres-the-twist", "Here's the door code.", 0, None),
            ("x-is-dead", "Peer code review is dead.", 1, None),
            (
                "x-is-dead",
                "The old importer is dead; long live the importer.",
                2,
                None,
            ),
            ("x-is-dead", "Long live the king.", 1, None),
            ("x-is-dead", "He played dead until the bear left.", 0, None),
            (
                "thats-why-mattered",
                "That's why being able to open the environment mattered.",
                1,
                None,
            ),
            (
                "thats-why-mattered",
                "This is why preserving every conversation mattered.",
                1,
                None,
            ),
            (
                "thats-why-mattered",
                "That's why the deadline counts.",
                1,
                None,
            ),
            ("thats-why-mattered", "That is why we left early.", 0, None),
            (
                "stranded-auxiliary",
                "The tool died; the data didn't.",
                1,
                None,
            ),
            (
                "stranded-auxiliary",
                "Reading mostly passed, writing didn't.",
                1,
                None,
            ),
            ("stranded-auxiliary", "Maybe it wouldn't have.", 1, None),
            (
                "stranded-auxiliary",
                "The test passed and the build was green.",
                0,
                None,
            ),
            (
                "ai-vocab",
                "We delve into the intricacies of the interplay.",
                3,
                None,
            ),
            (
                "ai-vocab",
                "Her vibrant tapestry hung in the bustling hall.",
                3,
                None,
            ),
            (
                "ai-vocab",
                "A meticulously curated, seamless experience.",
                2,
                None,
            ),
            (
                "ai-vocab",
                "The report was thorough and well organized.",
                0,
                None,
            ),
            (
                "not-just",
                "This is not just a tool, but a philosophy.",
                1,
                None,
            ),
            ("not-just", "Not only fast but also reliable.", 1, None),
            (
                "not-just",
                "It\u{2019}s not a bug \u{2014} it\u{2019}s a feature.",
                1,
                None,
            ),
            ("not-just", "He did not buy it.", 0, None),
            ("not-just", "She was not sure about the plan.", 0, None),
            (
                "note-that",
                "It is important to note that timing matters.",
                1,
                None,
            ),
            (
                "note-that",
                "It\u{2019}s worth noting the fees are separate.",
                1,
                None,
            ),
            (
                "note-that",
                "It should be noted that this changed in 2020.",
                1,
                None,
            ),
            ("note-that", "It's worth pausing on that number.", 1, None),
            ("note-that", "It is worth asking who benefits.", 1, None),
            ("note-that", "Please note the door code.", 0, None),
            (
                "testament",
                "The building stands as a testament to postwar optimism.",
                1,
                None,
            ),
            (
                "testament",
                "Her career is a testament to persistence.",
                1,
                None,
            ),
            (
                "testament",
                "It serves as a stark reminder that nothing lasts.",
                1,
                None,
            ),
            ("testament", "He read from the Old Testament.", 0, None),
            (
                "crucial-role",
                "Volunteers play a crucial role in the program.",
                1,
                None,
            ),
            (
                "crucial-role",
                "She played a truly pivotal role in the merger.",
                1,
                None,
            ),
            ("crucial-role", "He plays the role of the villain.", 0, None),
            (
                "landscape",
                "Adapting to an ever-evolving landscape.",
                1,
                None,
            ),
            (
                "landscape",
                "The rapidly changing landscape of retail.",
                1,
                None,
            ),
            (
                "landscape",
                "In today\u{2019}s fast-paced world, attention is scarce.",
                1,
                None,
            ),
            (
                "landscape",
                "The landscape outside the window was gray.",
                0,
                None,
            ),
            (
                "vague-experts",
                "Experts argue that the policy failed.",
                1,
                None,
            ),
            (
                "vague-experts",
                "Some critics have noted a decline in quality.",
                1,
                None,
            ),
            (
                "vague-experts",
                "Industry reports suggest strong demand.",
                1,
                None,
            ),
            (
                "vague-experts",
                "Dr. Chen argued the opposite in her paper.",
                0,
                None,
            ),
            (
                "despite-challenges",
                "Despite these challenges, growth continued.",
                1,
                None,
            ),
            (
                "despite-challenges",
                "The sector faces several challenges.",
                1,
                None,
            ),
            (
                "despite-challenges",
                "Whether it works remains to be seen.",
                1,
                None,
            ),
            (
                "despite-challenges",
                "Only time will tell whether it sticks.",
                1,
                None,
            ),
            ("despite-challenges", "Time will tell.", 1, None),
            (
                "despite-challenges",
                "He arrived on time and will tell you himself.",
                0,
                None,
            ),
            ("despite-challenges", "The climb was a challenge.", 0, None),
            (
                "participle-tail",
                "The bridge reopened in June, highlighting the city\u{2019}s investment in infrastructure.",
                1,
                None,
            ),
            (
                "participle-tail",
                "Sales doubled, underscoring the strength of the brand.",
                1,
                None,
            ),
            (
                "participle-tail",
                "She kept highlighting passages in yellow.",
                0,
                None,
            ),
            (
                "participle-tail",
                "The team, reflecting on the loss, regrouped.",
                0,
                None,
            ),
            ("promo", "The inn is nestled in a quiet valley.", 1, None),
            (
                "promo",
                "The museum boasts a rich tapestry of exhibits.",
                2,
                None,
            ),
            ("promo", "Located in the heart of downtown.", 1, None),
            ("promo", "A hidden gem with breathtaking views.", 2, None),
            ("promo", "The soup was rich and hearty.", 0, None),
            (
                "ai-leftovers",
                "As of my last update, the API was in beta.",
                1,
                None,
            ),
            (
                "ai-leftovers",
                "As an AI language model, I cannot form opinions.",
                1,
                None,
            ),
            (
                "ai-leftovers",
                "See example.com/page?utm_source=chatgpt.com for details.",
                1,
                None,
            ),
            (
                "ai-leftovers",
                "contentReference[oaicite:0]{index=0}",
                2,
                None,
            ),
            (
                "ai-leftovers",
                "The last update shipped on Tuesday.",
                0,
                None,
            ),
        ];
        let mut failures = Vec::new();
        for (id, sample, expect, items) in cases {
            let found = pattern(id)
                .unwrap_or_else(|| panic!("unknown pattern {id}"))
                .find(sample);
            if found.len() != *expect {
                failures.push(format!(
                    "{id} \u{b7} {sample:?}: expected {expect} matches, got {}",
                    found.len()
                ));
            }
            if let Some(items) = items {
                let counts: Vec<usize> = found.iter().map(|m| m.count.unwrap_or(0)).collect();
                if counts != *items {
                    failures.push(format!(
                        "{id} \u{b7} {sample:?}: expected counts {items:?}, got {counts:?}"
                    ));
                }
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    #[test]
    fn sentence_bounds_isolate_flagged_sentence() {
        let t = "First sentence here. No fluff, no filler. Last one.";
        let m = &pattern("no-chain").unwrap().find(t)[0];
        let (s, e) = sentence_bounds(t, m.start, m.end);
        assert_eq!(&t[s..e], "No fluff, no filler.");
    }

    fn words(prefix: &str, n: usize) -> String {
        (0..n)
            .map(|i| format!("{prefix}{i}"))
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn excerpt_context_is_twelve_words_each_side() {
        let t = format!(
            "{}. No fluff, no filler, just results. {}.",
            words("w", 30),
            words("t", 30)
        );
        let report = analyze(&t, &only("no-chain"));
        let wins = build_windows(&t, &report.regions);
        assert_eq!(wins.len(), 1);
        assert_eq!(count_words(&t[..wins[0].start]), 18);
        assert_eq!(count_words(&t[wins[0].end..]), 18);
    }

    #[test]
    fn nearby_matches_merge_into_one_window() {
        let t = format!(
            "{}. No fluff, no filler. Ok. No ads, no fees. {}.",
            words("w", 30),
            words("t", 30)
        );
        let report = analyze(&t, &only("no-chain"));
        let wins = build_windows(&t, &report.regions);
        assert_eq!(wins.len(), 1);
        assert_eq!(wins[0].regions.len(), 2);
    }

    #[test]
    fn distant_matches_stay_separate() {
        let t = format!("No fluff, no filler. {}. No ads, no fees.", words("m", 60));
        let report = analyze(&t, &only("no-chain"));
        let wins = build_windows(&t, &report.regions);
        assert_eq!(wins.len(), 2);
        assert_eq!(count_words(&t[wins[0].end..wins[1].start]), 36);
    }

    #[test]
    fn example_trips_every_pattern_once() {
        let report = analyze(EXAMPLE, &all());
        assert_eq!(report.matches.len(), PATTERNS.len());
        let distinct: HashSet<&str> = report.matches.iter().map(|m| m.pattern).collect();
        assert_eq!(distinct.len(), PATTERNS.len());
        assert_eq!(report.regions.len(), PATTERNS.len() - 1);
    }

    #[test]
    fn snippet_truncates_on_chars() {
        let long = "\u{e9}".repeat(100);
        assert_eq!(snippet(&long).chars().count(), 88);
        assert_eq!(snippet("  a \n b  "), "a b");
    }

    #[test]
    fn every_pattern_has_a_hint() {
        for p in PATTERNS.iter() {
            assert!(!p.hint.trim().is_empty(), "{} has no hint", p.id);
        }
    }
}
