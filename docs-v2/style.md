# How these pages are written

These pages use the core rules of ASD-STE100 (Simplified Technical English). They also use the four types of
the Diátaxis framework. This page gives the rules. Use it when you add or change a page in `docs-v2/`.

## The four types of page

| Folder | Type | Use it to |
| --- | --- | --- |
| `tutorials/` | Tutorial | Learn the product step by step, from the start to one good result. |
| `how-to/` | How-to guide | Do one specific job. |
| `reference/` | Reference | Find a fact: a key, a setting, a command, a part of the screen. |
| `explanation/` | Explanation | Understand how a part works and why it works that way. |
| `contributing/` | Contributor pages | Change the code of herdr-projects. |

Each page has one type. If a page starts to hold a second type, split it into two pages and link them.

## Writing rules (ASD-STE100, core rules)

### Sentences

1. Write procedural sentences (instructions) with a maximum of 20 words.
2. Write descriptive sentences with a maximum of 25 words.
3. Write one topic in each paragraph. Use a maximum of six sentences in each paragraph.
4. Use the active voice. Write "The board starts the agent", not "The agent is started by the board".
5. Use only these verb tenses: the simple present, the simple past and the simple future.
6. Do not use contractions. Write "do not", not "don't".
7. Use articles ("a", "an", "the") where English uses them. Do not remove them to make text shorter.
8. Do not use idioms or slang.
9. Do not use clusters of more than three nouns. Write "the time limit of a run", not "the run time limit value".

### Instructions

1. Write instructions in the imperative: "Press `Enter`."
2. Write one instruction in each step. You can write two actions in one step only if you do them at the same time.
3. Put a condition before the instruction: "If the agent stops, press `r`."
4. Use a numbered list for steps that have an order. Use a bulleted list for three or more items that have no order.
5. Put a note or a caution before the step that it is about, not after it.

### Words

1. Use one word for one meaning. Use the terms in [the glossary](reference/glossary.md) and do not use other words for them.
2. Use "must" for a rule. Use "can" for a possibility. Do not use "should" or "may" for a rule.
3. Use a single verb in place of a phrasal verb where possible: "remove", not "take out"; "start", not "fire up".
4. Use the names of keys, settings, commands, flags, files and folders exactly as the program uses them. Put them in
   `code` format.

### Notices

- **CAUTION:** You can lose data or work if you do not obey it.
- **Note:** More information that helps you. A note is never an instruction that you must obey.

## Page layout

1. Start each page with one `#` title.
2. Under the title, write one or two sentences that say what the page is for.
3. Link to related pages in a **Related pages** list at the end.
4. Use relative links between pages in `docs-v2/`.

## Generated tables

The key table and the settings table come from the code. `npm run check` writes them in `docs/keys.md` and
`docs/settings.md`. The copies in [reference/keys.md](reference/keys.md) and
[reference/settings.md](reference/settings.md) are snapshots. If a snapshot and `docs/` are different, `docs/` is
correct.
