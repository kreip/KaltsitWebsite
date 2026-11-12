import { defineMessages, type MessageMap } from "#/lib/i18n/messages";

export const namespace = "translators";

export const messages = {
    "meta.title": {
        text: "For translators",
        description: "Browser tab title of the translators' section (/translators).",
    },
    "meta.description": {
        text: "Story text side by side: English, Russian as the game shows it now, and Chinese.",
        description: "Search-engine description of the translators' section.",
    },
    "page.title": {
        text: "For translators",
        description: "Page heading of the translators' section.",
    },
    "tabs.label": {
        text: "Section views",
        description: "Accessible name of the tab strip of the translators' section.",
    },
    "tabs.story": {
        text: "Story",
        description: "Tab of the translators' section: chapter, story and its lines.",
    },
    "tabs.glossary": {
        text: "Glossary",
        description: "Tab of the translators' section: the team's whole glossary, read from its Google Sheet.",
    },
    "access.title": {
        text: "No access",
        description: "Heading shown on the translators' section to anyone who is not a member of the translation team.",
    },
    "access.description": {
        text: "This section is for the Russian translation team. Sign in and ask the team to add your account.",
        description: "Line under the No access heading.",
    },
    "load.access": {
        text: "Couldn't check your access to the section.",
        description: "Load failure of the membership check of the translators' section.",
    },
    "picker.chapter": {
        text: "Chapter",
        description: "Label of the chapter (story group) search box.",
    },
    "picker.chapterPlaceholder": {
        text: "Search {count, plural, one {# chapter} other {# chapters}}",
        description: "Placeholder of the chapter search box; count is the number of chapters.",
    },
    "picker.chapterEmpty": {
        text: "No chapters match.",
        description: "Shown in the chapter search list when nothing matches the typed text.",
    },
    "picker.story": {
        text: "Story",
        description: "Label of the story select inside the chosen chapter.",
    },
    "picker.storyPlaceholder": {
        text: "Choose a story",
        description: "Story select before a story is chosen.",
    },
    "category.main": { text: "Main story", description: "Chapter kind in the chapter list." },
    "category.side": { text: "Event", description: "Chapter kind in the chapter list." },
    "category.vignette": { text: "Side story", description: "Chapter kind in the chapter list." },
    "category.record": { text: "Operator record", description: "Chapter kind in the chapter list." },
    "category.is": { text: "Integrated Strategies", description: "Chapter kind in the chapter list." },
    "category.reclamation": { text: "Reclamation Algorithm", description: "Chapter kind in the chapter list." },
    "category.sideContent": { text: "Other", description: "Chapter kind in the chapter list." },
    "toggle.cn": {
        text: "Chinese",
        description: "Switch that shows the Chinese column of the table.",
    },
    "toggle.cnHint": {
        text: "The Chinese original.",
        description: "Second line under the Chinese switch.",
    },
    "toggle.service": {
        text: "Service",
        description: "Switch that shows script line numbers and the script's stage commands (scene, background, pause…) between the lines.",
    },
    "toggle.serviceHint": {
        text: "Music, background, who is on stage.",
        description: "Second line under the Service switch.",
    },
    "toggle.variants": {
        text: "Show variants",
        description: "Switch that shows Sova's and the AI's translations under each Russian line.",
    },
    "toggle.variantsHint": {
        text: "Sova and machine translation under the Russian.",
        description: "Second line under the Show variants switch.",
    },
    "book.back": {
        text: "Back to the section",
        description: "Button above the book view opened from the translators' section: returns to the same chapter and story.",
    },
    "book.open": {
        text: "Book view",
        description: "Link that opens the chosen story in the site's book view.",
    },
    "table.label": {
        text: "Lines of {story}",
        description: "Accessible name of the lines table; story is the story's title.",
    },
    "table.line": { text: "#", description: "Column: the line's number in the story (1, 2, 3…)." },
    "table.scriptLine": {
        text: "line {line}",
        description: "Under a line's number, and before a stage command, when Service is on: its line in the English script file.",
    },
    "table.speaker": { text: "Speaker", description: "Column: who says the line." },
    "table.en": { text: "English", description: "Column: the English text." },
    "table.ru": { text: "Russian (in game)", description: "Column: the Russian the game shows now." },
    "table.cn": { text: "Chinese", description: "Column: the Chinese original." },
    "table.summary": {
        text: "{count, plural, one {# line} other {# lines}}, {ru} with Russian",
        description: "Count above the lines table; ru is how many have a Russian text.",
    },
    "line.option": {
        text: "Choice {part}",
        description: "Badge of a reply option of the player's decision in the lines table.",
    },
    "line.subtitle": { text: "Caption", description: "Label of a subtitle text (shown over the scene) in the lines table." },
    "line.sticker": { text: "On-screen text", description: "Label of a sticker text in the lines table." },
    "ru.missing": {
        text: "No translation found",
        description: "Russian cell when no dictionary line matched the English text.",
    },
    "ru.lineByLine": {
        text: "Assembled line by line",
        description: "Note under a Russian text that was found one line at a time.",
    },
    "plugin.badge": {
        text: "Plugin",
        description: "Badge on a line whose Russian comes from the RichTextFix plugin's built-in catalog.",
    },
    "plugin.hint": {
        text: "Shown by the plugin: an edit may not take effect.",
        description: "Explanation behind the Plugin badge.",
    },
    "ambiguous.badge": {
        text: "Left in English",
        description: "Badge on a line the plugin deliberately shows untranslated.",
    },
    "ambiguous.hint": {
        text: "The game keeps this line in English on purpose.",
        description: "Explanation behind the Left in English badge.",
    },
    "service.label": {
        text: "Stage commands",
        description: "Accessible name of a gray separator row listing the script's stage commands between two lines.",
    },
    "terms.label": {
        text: "Glossary terms in this line",
        description: "Accessible name of the list of glossary terms found in a line's English.",
    },
    "terms.found": {
        text: "form found",
        description: "Mark beside a glossary term whose Russian form is in the line's Russian.",
    },
    "terms.notFound": {
        text: "form not found",
        description: "Mark beside a glossary term whose Russian form is not in the line's Russian (by word stem). Not an error.",
    },
    "terms.variant": {
        text: "glossary",
        description: "Mark beside a term from the sheet copied from Sova's glossary: a variant, never checked.",
    },
    "hints.sova": { text: "Sova", description: "Label of Sova's translation of a line." },
    "hints.ai": { text: "Machine translation", description: "Label of the machine translation of a line." },
    "hints.dict": {
        text: "Dictionary (overridden by the plugin)",
        description: "Label of the dictionary's own Russian when the plugin shows a different one.",
    },
    "devLogin.link": {
        text: "Development sign-in",
        description: "Link to the development sign-in page, shown only when it is switched on.",
    },
    "empty.choose": {
        text: "Choose a chapter and a story.",
        description: "Shown before a story is chosen.",
    },
    "empty.notImported": {
        text: "The story text has not been imported yet.",
        description: "Shown when the section's database is empty.",
    },
    "empty.noLines": {
        text: "This story has no text lines.",
        description: "Shown for a story with nothing to translate.",
    },
    "load.index": {
        text: "Couldn't load the story list.",
        description: "Load failure of the chapter list.",
    },
    "load.lines": {
        text: "Couldn't load the story lines.",
        description: "Load failure of the lines table.",
    },
    "glossary.load": {
        text: "Couldn't load the glossary.",
        description: "Load failure of the Glossary tab.",
    },
    "glossary.copy": {
        text: "Copy downloaded {date}",
        description: "Date of the stored copy of the glossary.",
    },
    "glossary.none": {
        text: "The glossary hasn't been downloaded yet.",
        description: "Shown on the Glossary tab when no copy is stored.",
    },
    "glossary.failed": {
        text: "The last download failed ({date}). The stored copy is shown.",
        description: "Warning when the last download of the glossary failed; date is the attempt's time.",
    },
    "glossary.refresh": {
        text: "Refresh glossary",
        description: "Button that downloads the glossary again from the team's Google Sheet.",
    },
    "glossary.refreshing": {
        text: "Downloading…",
        description: "The refresh button while the download runs.",
    },
    "glossary.open": {
        text: "Open in the sheet",
        description: "Link to the team's Google Sheet, where the glossary is edited.",
    },
    "glossary.readOnly": {
        text: "Edits are made in the sheet only.",
        description: "Note beside the link to the sheet.",
    },
    "glossary.search": {
        text: "Search English or Russian",
        description: "Placeholder of the glossary search box.",
    },
    "glossary.searchLabel": {
        text: "Search the glossary",
        description: "Accessible name of the glossary search box.",
    },
    "glossary.clear": {
        text: "Clear search",
        description: "Button that empties the glossary search box.",
    },
    "glossary.sheet": {
        text: "Sheet",
        description: "Label of the sheet filter of the glossary, and its column.",
    },
    "glossary.sheetAll": {
        text: "All sheets",
        description: "Sheet filter option: every sheet.",
    },
    "glossary.sort": {
        text: "Sort",
        description: "Label of the glossary sort select.",
    },
    "glossary.sort.sheet": { text: "As in the sheet", description: "Sort option: sheet and row order." },
    "glossary.sort.en": { text: "English A–Z", description: "Sort option: by the English term." },
    "glossary.sort.ru": { text: "Russian А–Я", description: "Sort option: by the Russian term." },
    "glossary.sort.count": { text: "Most used in the story", description: "Sort option: by how many story lines hold the term." },
    "glossary.col.en": { text: "English", description: "Glossary column: the English term." },
    "glossary.col.ru": { text: "Russian", description: "Glossary column: how to translate it." },
    "glossary.col.notes": { text: "Notes", description: "Glossary column: the sheet's notes, synonyms and comment." },
    "glossary.col.count": { text: "In story", description: "Glossary column: how many story lines hold the term." },
    "glossary.plural": {
        text: "pl. {form}",
        description: "The Russian plural of a race or similar term; form is the plural.",
    },
    "glossary.summary": {
        text: "{shown, plural, one {# term} other {# terms}} of {total}",
        description: "Count above the glossary table.",
    },
    "glossary.empty": {
        text: "No terms match.",
        description: "Shown when the search and filter leave no terms.",
    },
    "glossary.tableLabel": {
        text: "Glossary",
        description: "Accessible name of the glossary table.",
    },
    "dev.title": {
        text: "Development sign-in",
        description: "Heading of the development sign-in page.",
    },
    "dev.button": {
        text: "Sign in as translator",
        description: "Button on the development sign-in page.",
    },
    "dev.off": {
        text: "Development sign-in is switched off on this backend.",
        description: "Shown on the development sign-in page when the backend does not allow it.",
    },
    "dev.failed": {
        text: "Couldn't sign in.",
        description: "Shown when the development sign-in request fails.",
    },
    "tabs.edits": {
        text: "My edits",
        description: "Tab of the translators' section: the member's own proposals (everyone's for the lead), with history.",
    },
    "status.unchecked": {
        text: "Not checked",
        description: "Status of a story line nobody has proposed or decided anything for.",
    },
    "status.proposed": {
        text: "Has proposals",
        description: "Status of a story line with at least one translator's proposal; shown as a dot by the line number.",
    },
    "status.approved": {
        text: "Approved",
        description: "Status of a story line whose translation the lead approved.",
    },
    "status.outdated": {
        text: "Outdated",
        description: "Status of a decision made for an English text the story no longer has at that place (it changed in an import).",
    },
    "approved.label": {
        text: "Approved by {name}, {date}",
        description: "Label over the approved translation of a line; name is the lead, date when.",
    },
    "approved.game": {
        text: "Checked as in the game by {name}, {date}",
        description: "Label over a line whose current game translation the lead checked and kept.",
    },
    "proposal.mine": {
        text: "My proposal",
        description: "Author label of the viewer's own proposal for a line.",
    },
    "proposal.edited": {
        text: "edited",
        description: "Marks a proposal that was rewritten after it was first saved.",
    },
    "proposal.from.sova": {
        text: "from Sova's",
        description: "Marks a proposal that started from Sova's translation.",
    },
    "proposal.from.ai": {
        text: "from machine translation",
        description: "Marks a proposal that started from the AI's translation.",
    },
    "proposal.from.kit": {
        text: "from the review kit",
        description: "Marks a proposal that came from the review kit's reports (reserved).",
    },
    "proposal.from.own": {
        text: "own text",
        description: "Marks a proposal written by the translator.",
    },
    "proposal.agreed": {
        text: "{count, plural, one {# agrees} other {# agree}}",
        description: "How many translators agree with a proposal.",
    },
    "proposal.edit": {
        text: "Edit",
        description: "Button under the viewer's own proposal in the story table: opens it in the editor.",
    },
    "proposal.rewrite": {
        text: "Rewrite",
        description: "Button in My edits: opens the proposal's text for rewriting.",
    },
    "proposal.delete": {
        text: "Delete",
        description: "Button deleting the viewer's own proposal.",
    },
    "edit.failedSave": {
        text: "Couldn't save the proposal.",
        description: "Toast when saving a proposal fails.",
    },
    "edit.failedDelete": {
        text: "Couldn't delete the proposal.",
        description: "Toast when deleting a proposal fails.",
    },
    "outdated.proposal": {
        text: "{name}'s proposal for the previous English",
        description: "Label of a proposal left at a line whose English has since changed; name is its author.",
    },
    "outdated.approval": {
        text: "Approved by {name} for the previous English",
        description: "Label of an approval left at a line whose English has since changed.",
    },
    "outdated.was": {
        text: "Was for: {en}",
        description: "The old English text an outdated decision was made for.",
    },
    "comment.failed": {
        text: "Couldn't save the comment.",
        description: "Toast when adding, resolving or deleting a comment fails.",
    },
    "comment.resolve": {
        text: "Resolved",
        description: "Button marking a comment as resolved (it then folds away).",
    },
    "comment.reopen": {
        text: "Reopen",
        description: "Button marking a resolved comment as open again.",
    },
    "comment.delete": {
        text: "Delete",
        description: "Button deleting the viewer's own comment.",
    },
    "comment.resolvedBy": {
        text: "resolved by {name}",
        description: "Marks a resolved comment; name is who resolved it.",
    },
    "comment.resolvedCount": {
        text: "{count, plural, one {# resolved comment} other {# resolved comments}}",
        description: "Button that unfolds the resolved comments of a line.",
    },
    "editor.label": {
        text: "My translation",
        description: "Label of the editor of the viewer's own proposal.",
    },
    "editor.label.sova": {
        text: "My translation, from Sova's",
        description: "Label of the editor when it started from Sova's translation.",
    },
    "editor.label.ai": {
        text: "My translation, from machine translation",
        description: "Label of the editor when it started from the AI's translation.",
    },
    "editor.rewrite": {
        text: "Rewrite my proposal",
        description: "Label of the rewrite box in My edits.",
    },
    "editor.wholeEntry": {
        text: "All its lines are saved together.",
        description: "Under the editor of a multi-line story text.",
    },
    "editor.sameAsGame": {
        text: "This is the game's text as it is: there is nothing to propose.",
        description: "Under the editor when the text equals the game's current Russian (saving is off).",
    },
    "editor.save": {
        text: "Save",
        description: "Button saving the proposal.",
    },
    "editor.cancel": {
        text: "Cancel",
        description: "Button closing the editor without saving.",
    },
    "editor.keys": {
        text: "Ctrl+Enter saves, Esc cancels",
        description: "Keyboard hint under the editor.",
    },
    "places.note": {
        text: "This line also stands in {count, plural, one {# other place} other {# other places}}: the edit applies to all.",
        description: "Warning under the editor when the same dictionary entry appears elsewhere in the story.",
    },
    "places.show": {
        text: "Show places",
        description: "Button unfolding the list of the places of a line.",
    },
    "places.load": {
        text: "Couldn't load the places of this line.",
        description: "Load failure of the list of places of a line.",
    },
    "places.item": {
        text: "{group} › {story} · line {n}",
        description: "One place of a line: chapter › story · line number in the story table.",
    },
    "places.itemBare": {
        text: "{story} · line {n}",
        description: "One place of a line whose chapter is unknown.",
    },
    "places.here": {
        text: "this line",
        description: "Marks the current line in the list of its places.",
    },
    "places.more": {
        text: "and {count} more",
        description: "Under a cut list of places.",
    },
    "where.item": {
        text: "Story › {group} › {story} › line {n}",
        description: "Where a proposal was made, in My edits.",
    },
    "where.places": {
        text: "in {count} places",
        description: "Under the place in My edits: how many story lines hold the same entry.",
    },
    "history.open": {
        text: "History ({count})",
        description: "Button unfolding the texts a proposal has had; count is how many.",
    },
    "history.label": {
        text: "Texts of this proposal, newest first",
        description: "Accessible name of a proposal's history list.",
    },
    "history.current": {
        text: "current",
        description: "Marks the current text in a proposal's history.",
    },
    "history.load": {
        text: "Couldn't load the history.",
        description: "Load failure of a proposal's history.",
    },
    "state.proposed": {
        text: "Proposed",
        description: "State of a proposal in My edits: waiting for the lead.",
    },
    "state.approved": {
        text: "Approved",
        description: "State of a proposal the lead approved.",
    },
    "state.otherApproved": {
        text: "Another approved",
        description: "State of a proposal when the lead approved another variant for the line.",
    },
    "state.outdated": {
        text: "Outdated",
        description: "State of a proposal whose English is gone from the story since an import.",
    },
    "edits.all": {
        text: "Everyone's",
        description: "Switch for the lead in My edits: show every translator's proposals.",
    },
    "edits.load": {
        text: "Couldn't load the edits.",
        description: "Load failure of My edits.",
    },
    "edits.empty": {
        text: "No proposals yet",
        description: "Heading of the empty My edits tab.",
    },
    "edits.emptyHint": {
        text: "Open a story and press Propose on a line.",
        description: "Hint under the empty My edits heading.",
    },
    "edits.count": {
        text: "{count, plural, one {# proposal} other {# proposals}}",
        description: "Above the My edits table.",
    },
    "edits.tableLabel": {
        text: "Proposals",
        description: "Accessible name of the My edits table.",
    },
    "edits.col.was": {
        text: "Was (in the game)",
        description: "My edits column: the game's Russian now and its English.",
    },
    "edits.col.now": {
        text: "Now (proposed)",
        description: "My edits column: the proposal's text.",
    },
    "edits.col.where": {
        text: "Where",
        description: "My edits column: the place of the proposal.",
    },
    "edits.col.state": {
        text: "State",
        description: "My edits column: proposed, approved, outdated.",
    },
    "edits.col.date": {
        text: "Date",
        description: "My edits column: when the proposal was last saved.",
    },
    "nickname.fallback": {
        text: "Doctor",
        description: "Shown for {@nickname} in story text when the signed-in member has no game nickname.",
    },
    "glossary.sovaSheet": {
        text: "Glossary",
        description: "Name shown for the glossary sheet copied from Sova's glossary; its terms are variants, not approved.",
    },
    "action.propose": {
        text: "Propose",
        description: "Button at the top right of a line's Russian (on hover): opens the editor for a proposed translation.",
    },
    "action.proposeThis": {
        text: "Propose this",
        description: "Small button under Sova's or the AI's variant: opens the editor filled with that variant.",
    },
    "action.remark": {
        text: "Remark",
        description: "Button on a line: a remark (a question or a problem) that can later be marked resolved.",
    },
    "action.note": {
        text: "Note",
        description: "Button on a line: a translator's note (why it is translated so, a reference, a fact); never resolved.",
    },
    "proposal.by": {
        text: "{name}",
        description: "Author label over another translator's proposal; name is the author.",
    },
    "proposal.contextBadge": {
        text: "This place only",
        description: "Badge on a proposal made for one place of the line only (not for every place of the same English).",
    },
    "remark.label": {
        text: "Remark",
        description: "Label of a remark under a line.",
    },
    "remark.editorLabel": {
        text: "Remark on this line",
        description: "Label of the remark box.",
    },
    "remark.save": {
        text: "Add remark",
        description: "Button saving a remark.",
    },
    "note.label": {
        text: "Translator's note",
        description: "Label of a translator's note under a line or a variant.",
    },
    "note.editorLabel": {
        text: "Translator's note",
        description: "Label of the note box.",
    },
    "note.save": {
        text: "Add note",
        description: "Button saving a translator's note.",
    },
    "note.about": {
        text: "About",
        description: "Label of the select choosing what a note is about: the line or one variant.",
    },
    "note.aboutLine": {
        text: "The whole line",
        description: "Option: the note is about the line.",
    },
    "note.aboutGame": {
        text: "The Russian as in the game",
        description: "Option: the note is about the game's current translation.",
    },
    "note.aboutProposal": {
        text: "{name}'s proposal",
        description: "Option: the note is about a translator's proposal; name is its author.",
    },
    "note.aboutSova": {
        text: "Sova's variant",
        description: "Option: the note is about Sova's translation.",
    },
    "note.aboutAi": {
        text: "Machine translation",
        description: "Option: the note is about the AI's translation.",
    },
    "note.forGone": {
        text: "About a variant that is no longer here: {text}",
        description: "Above a note whose variant was deleted or replaced; text is the variant as it was.",
    },
    "comment.quote": {
        text: "On: “{quote}”",
        description: "The fragment of the line a remark or note is about.",
    },
    "comment.quoteHint": {
        text: "Select a fragment first to comment on it.",
        description: "Hint in the remark or note box when no fragment is selected.",
    },
    "comment.dropQuote": {
        text: "Whole line",
        description: "Button removing the selected fragment from a new remark or note.",
    },
    "places.before": {
        text: "Before:",
        description: "Label of the line before, in the list of the places of a line.",
    },
    "places.after": {
        text: "After:",
        description: "Label of the line after, in the list of the places of a line.",
    },
    "save.title": {
        text: "This line stands in {count} places",
        description: "Heading of the dialog when saving a proposal for a line whose English appears in several places.",
    },
    "save.proposalOnly": {
        text: "This is a proposal. It reaches the game only after the lead approves it.",
        description: "Shown under the editor and in the save dialog.",
    },
    "save.scope": {
        text: "Where the proposal applies",
        description: "Accessible name of the choice between all places and this place only.",
    },
    "save.all": {
        text: "For all {count} places",
        description: "Option of the save dialog: one translation of this English everywhere.",
    },
    "save.allHint": {
        text: "The game has one translation for this text in every place.",
        description: "Hint under the all-places option.",
    },
    "save.here": {
        text: "Only for this place",
        description: "Option of the save dialog: a translation tied to this place.",
    },
    "save.hereHint": {
        text: "A translation for this line only.",
        description: "Hint under the this-place-only option.",
    },
    "save.understand": {
        text: "I understand that I change the translation in all {count} places",
        description: "Checkbox the member must tick before saving for all places.",
    },
    "save.saveAll": {
        text: "Save for {count} places",
        description: "Button saving a proposal for all places.",
    },
    "save.saveHere": {
        text: "Save for this place",
        description: "Button saving a proposal for this place only.",
    },
    "editor.insertNickname": {
        text: "Insert the nickname",
        description: "Button under the editor: inserts the {@nickname} chip at the cursor.",
    },
    "editor.contextOnly": {
        text: "This proposal is for this place only.",
        description: "Under the editor of a one-place proposal.",
    },
    "sovaNotes.title": {
        text: "Sova's notes to the chapter ({count})",
        description: "Button unfolding Sova's footnotes to the chapter, above the story table.",
    },
    "sovaNotes.author": {
        text: "Sova",
        description: "Author label of one of Sova's notes.",
    },
    "service.music": {
        text: "Music: {name}",
        description: "Stage command in words: music starts; name is the track key.",
    },
    "service.musicStop": {
        text: "Music stops",
        description: "Stage command in words.",
    },
    "service.sound": {
        text: "Sound: {name}",
        description: "Stage command in words: a sound plays; name is its key.",
    },
    "service.soundStop": {
        text: "Sound stops",
        description: "Stage command in words.",
    },
    "service.background": {
        text: "Background: {name}",
        description: "Stage command in words: the background changes; name is the image key.",
    },
    "service.backgroundClear": {
        text: "Background removed",
        description: "Stage command in words.",
    },
    "service.image": {
        text: "Picture: {name}",
        description: "Stage command in words: a full-screen picture shows.",
    },
    "service.imageClear": {
        text: "Picture removed",
        description: "Stage command in words.",
    },
    "service.onStage": {
        text: "On stage: {names}",
        description: "Stage command in words: who is shown; names are the characters.",
    },
    "service.stageClear": {
        text: "Stage empty",
        description: "Stage command in words: the characters leave.",
    },
    "service.fadeOut": {
        text: "Fade out",
        description: "Stage command in words: the screen darkens.",
    },
    "service.fadeIn": {
        text: "Fade in",
        description: "Stage command in words: the screen appears again.",
    },
    "service.shake": {
        text: "Camera shake",
        description: "Stage command in words.",
    },
    "service.cameraEffect": {
        text: "Camera effect",
        description: "Stage command in words.",
    },
    "service.pause": {
        text: "Pause",
        description: "Stage command in words.",
    },
    "service.branch": {
        text: "Branch: choice {options}",
        description: "Stage command in words: the following lines are shown after the given choice.",
    },
    "service.video": {
        text: "Video",
        description: "Stage command in words.",
    },
    "service.volume": {
        text: "Volume",
        description: "Stage command in words.",
    },
    "service.other": {
        text: "{name}",
        description: "Stage command the section does not describe: its name only.",
    },
    "glossary.col.game": {
        text: "As in the game",
        description: "Glossary column: the Russian words the game uses for the term, with the number of lines.",
    },
    "glossary.col.consistency": {
        text: "Consistency",
        description: "Glossary column: the share of the term's lines whose Russian holds the glossary's form.",
    },
    "glossary.col.cn": {
        text: "CN",
        description: "Glossary column: the term's Chinese.",
    },
    "glossary.consistencyHint": {
        text: "{found} of {checked} lines hold the glossary's form",
        description: "Tooltip of the consistency value.",
    },
    "glossary.cnFromGame": {
        text: "From the game's speaker names",
        description: "Tooltip of a Chinese name taken from the story's speakers.",
    },
    "glossary.variantGlossary": {
        text: "The glossary's form",
        description: "Marks the game's word that is the glossary's form.",
    },
    "glossary.sharedHint": {
        text: "Same English as “{key}”: the lines are counted there.",
        description: "Tooltip of the dash for a glossary entry spelled like another.",
    },
    "glossary.showLines": {
        text: "Show lines",
        description: "Button opening the story lines of a glossary term.",
    },
    "glossary.sort.consistency": {
        text: "Least consistent first",
        description: "Glossary sort option.",
    },
    "glossary.lines.title": {
        text: "Lines with “{term}”",
        description: "Heading of the dialog listing a term's story lines.",
    },
    "glossary.lines.description": {
        text: "Glossary: {ru}",
        description: "Line under the heading: the glossary's translation.",
    },
    "glossary.lines.missing": {
        text: "Only lines without the glossary's form",
        description: "Switch in the term lines dialog.",
    },
    "glossary.lines.count": {
        text: "{shown} of {total}",
        description: "How many lines are listed out of all.",
    },
    "glossary.lines.load": {
        text: "Couldn't load the lines.",
        description: "Load failure of a term's lines.",
    },
    "glossary.lines.empty": {
        text: "No lines.",
        description: "Shown when a term has no lines (or none without the form).",
    },
    "checks.recommended": {
        text: "recommended",
        description: "Badge on the one variant of a line that passes every check and wins by the rule (most agreed proposal, else Sova's, the AI's, the game's).",
    },
    "checks.recommendedHint": {
        text: "The best variant without errors.",
        description: "Tooltip of the recommended badge.",
    },
    "checks.glossaryHint": {
        text: "The glossary's form is missing: {terms}",
        description: "Tooltip of the missing glossary terms of a variant; terms are 'English → Russian' pairs.",
    },
    "checks.tags": {
        text: "tags",
        description: "Short mark: the variant's markup tags (italics, colour) differ from the English.",
    },
    "checks.tagsHint": {
        text: "Italics or colour differ from the English.",
        description: "Tooltip of the tags mark.",
    },
    "checks.placeholdersHint": {
        text: "A substitution ({0}, nickname) differs from the English.",
        description: "Tooltip of the substitutions mark; the mark itself lists them.",
    },
    "checks.newlines": {
        text: "line breaks {ru} / {en}",
        description: "Short mark: the variant's line breaks differ from the English (variant's count / English's).",
    },
    "checks.newlinesHint": {
        text: "Line breaks: {ru} here, {en} in the English.",
        description: "Tooltip of the line-breaks mark.",
    },
    "checks.longer": {
        text: "+{share}",
        description: "Short mark: the variant is noticeably longer than the game's Russian; share is how much longer (a percentage).",
    },
    "checks.longerHint": {
        text: "Longer than in the game: may not fit the dialogue box.",
        description: "Tooltip of the length mark.",
    },
    "checks.address": {
        text: "“{form}”",
        description: "Short mark: the variant addresses with this form, unlike the speaker's neighbouring lines.",
    },
    "checks.addressHint": {
        text: "The speaker's neighbouring lines in this scene say “{form}”.",
        description: "Tooltip of the ты / вы mark; form is what the neighbouring lines use.",
    },
    "service.guessed": {
        text: "{name} (presumably)",
        description: "A story sprite's name guessed from who talks after it appears (not an operator).",
    },
    "agree.label": {
        text: "Agree",
        description: "Accessible name of the agree button of a variant (a thumbs-up with the count).",
    },
    "agree.hint": {
        text: "Agree with this variant",
        description: "Tooltip of the agree button when nobody has agreed yet.",
    },
    "agree.who": {
        text: "{count, plural, one {# agrees} other {# agree}}: {names}",
        description: "Tooltip of the agree button: how many and who.",
    },
    "read.button": {
        text: "I have read this scene",
        description: "Toggle above the story table: the member marks the story read.",
    },
    "read.by": {
        text: "Read: {names}",
        description: "Who has read the story, with dates; names is a comma-separated list.",
    },
    "read.item": {
        text: "{name} ({date})",
        description: "One reader of the story and when.",
    },
    "read.failed": {
        text: "Couldn't save the mark.",
        description: "Toast when marking a story read fails.",
    },
    "progress.label": {
        text: "Progress of the story",
        description: "Accessible name of the story's progress meter.",
    },
    "progress.summary": {
        text: "Checked {checked} · waiting {waiting} · outdated {outdated}",
        description: "Story progress above the table; checked is a percentage, waiting and outdated are line counts.",
    },
    "progress.short": {
        text: "{checked} checked · {waiting} waiting · {outdated} outdated",
        description: "Short progress under a chapter or story name in the pickers.",
    },
    "agree.failed": {
        text: "Couldn't save your vote.",
        description: "Toast when agreeing with a variant (or taking it back) fails.",
    },
    "approve.button": {
        text: "Approve",
        description: "The lead's button on a variant: approve it for the game (the game's Russian: checked, kept).",
    },
    "approve.withdraw": {
        text: "Withdraw approval",
        description: "The lead's button on the approved variant.",
    },
    "approve.failed": {
        text: "Couldn't save the approval.",
        description: "Toast when approving or withdrawing fails.",
    },
    "approved.here": {
        text: "approved for this place",
        description: "Badge on a one-place proposal the lead approved.",
    },
    "filter.label": {
        text: "Show",
        description: "Label of the filter above the story table.",
    },
    "filter.all": {
        text: "All lines",
        description: "Filter option: every line.",
    },
    "filter.proposals": {
        text: "With proposals",
        description: "Filter option: lines with translators' proposals.",
    },
    "filter.recommended": {
        text: "With a recommendation",
        description: "Filter option: lines where a variant is recommended (the game's text has a mark).",
    },
    "filter.outdated": {
        text: "Outdated",
        description: "Filter option: lines with a decision left for a previous English.",
    },
    "filter.remarks": {
        text: "With open remarks",
        description: "Filter option: lines with remarks not yet resolved.",
    },
    "filter.count": {
        text: "{shown} of {total}",
        description: "How many lines the filter shows.",
    },
    "bulk.recommended": {
        text: "Approve the recommended in the whole scene…",
        description: "The lead's button above the table: opens the list of recommended variants to approve.",
    },
    "bulk.game": {
        text: "Whole scene as in the game, checked…",
        description: "The lead's button above the table: marks the game's Russian of the scene's lines checked.",
    },
    "bulk.recommendedTitle": {
        text: "Approve {count, plural, one {# recommended variant} other {# recommended variants}}",
        description: "Heading of the dialog approving a scene's recommended variants.",
    },
    "bulk.recommendedHint": {
        text: "Untick the lines to leave out.",
        description: "Text of the dialog approving recommended variants.",
    },
    "bulk.gameTitle": {
        text: "Mark the game's Russian checked in {count, plural, one {# line} other {# lines}}",
        description: "Heading of the dialog marking a scene as in the game.",
    },
    "bulk.gameHint": {
        text: "The game's text stays; approved lines are not changed.",
        description: "Text of the dialog marking a scene as in the game.",
    },
    "bulk.gameSkipped": {
        text: "Left out: {proposals} with proposals, {marks} with a mark on the game's text.",
        description: "In the as-in-the-game dialog: how many lines are left out by default.",
    },
    "bulk.gameInclude": {
        text: "Include them too",
        description: "Checkbox in the as-in-the-game dialog: also mark the left-out lines checked.",
    },
    "bulk.confirm": {
        text: "Approve",
        description: "Button confirming a batch approval.",
    },
    "bulk.done": {
        text: "Approved: {approved}, skipped: {skipped}.",
        description: "Toast after a batch approval.",
    },
    "bulk.failed": {
        text: "Couldn't approve the scene.",
        description: "Toast when a batch approval fails.",
    },
    "export.open": {
        text: "Export…",
        description: "Button in My edits opening the export to the XUnity dictionaries.",
    },
    "export.title": {
        text: "Export to the XUnity dictionaries",
        description: "Heading of the export dialog.",
    },
    "export.mode": {
        text: "What",
        description: "Label of the export mode choice.",
    },
    "export.mode.approved": {
        text: "Approved — for the game",
        description: "Export mode: the lead's approvals (the lead only).",
    },
    "export.mode.approvedHint": {
        text: "Only the lead or the pack maintainer can export it.",
        description: "Hint under the approved mode for other members.",
    },
    "export.mode.review": {
        text: "All proposals — for checking (REVIEW)",
        description: "Export mode: every proposal not approved, marked REVIEW, not for the game.",
    },
    "export.format": {
        text: "Format",
        description: "Label of the export format choice.",
    },
    "export.format.file": {
        text: "One file 00_translators.txt — to check at home",
        description: "Export format: a separate dictionary file.",
    },
    "export.format.patch": {
        text: "Corrected copies of 11_ui, 12_story, 13_names (main)",
        description: "Export format: the pack's files with the changed values.",
    },
    "export.run": {
        text: "Prepare",
        description: "Button preparing the export.",
    },
    "export.load": {
        text: "Couldn't prepare the export.",
        description: "Load failure of the export.",
    },
    "export.written": {
        text: "Lines changed in the dictionaries: {count}",
        description: "Export report: lines written.",
    },
    "export.plugin": {
        text: "RichTextFix plugin strings (plugin_strings.tsv): {count}",
        description: "Export report: plugin lines.",
    },
    "export.skipped": {
        text: "Left out (XUnity cannot read them): {count}",
        description: "Export report: unwritable keys.",
    },
    "export.after": {
        text: "Loaded after ours (they win on the same key): {files}",
        description: "Export report: files that override ours.",
    },
    "export.afterNone": {
        text: "No file loads after ours: it wins.",
        description: "Export report: nothing overrides ours.",
    },
    "export.files": {
        text: "Files",
        description: "Heading of the list of exported files.",
    },
    "export.download": {
        text: "Download",
        description: "Link downloading one exported file.",
    },
    "export.howTo": {
        text: "Put the file in BepInEx\\Translation\\ru\\Text and press Alt+R in the game.",
        description: "Hint in the export dialog: how to try an exported dictionary file in the game. Keep the folder path and the key as they are.",
    },
    "release.in": {
        text: "went out in pack {version}",
        description: "Badge on an approved variant whose text went out in a pack version.",
    },
    "release.title": {
        text: "Went out in a pack",
        description: "Heading of the pack maintainer's block in the export dialog.",
    },
    "release.hint": {
        text: "Mark it once the pack from this export is out.",
        description: "Text of the pack maintainer's block.",
    },
    "release.version": {
        text: "Pack version",
        description: "Label of the pack version field.",
    },
    "release.note": {
        text: "Note",
        description: "Label of the optional note of a pack release.",
    },
    "release.mark": {
        text: "Mark as gone out",
        description: "Button marking the approved entries as gone out in the pack version.",
    },
    "release.done": {
        text: "Marked in {version}: dictionary {dictionary}, plugin {plugin}, tied to a place {context}.",
        description: "After marking a release: how many entries of each kind.",
    },
    "release.last": {
        text: "Last: {version} ({date}, {by})",
        description: "The last pack version marked.",
    },
    "release.failed": {
        text: "Couldn't mark the release.",
        description: "Toast when marking a pack release fails.",
    },
    "export.format.patchHint": {
        text: "The pack's files with the approved edits, changed ones only.",
        description: "Hint under the main export format.",
    },
    "export.format.fileHint": {
        text: "Drop it into the game and press Alt+R.",
        description: "Hint under the to-check-at-home export format.",
    },
    "export.story": {
        text: "Story lines for the bundles (story_lines.tsv): {count}, of them for one place: {context}",
        description: "Export report: rows of story_lines.tsv and how many are one-place proposals.",
    },
    "export.howToPatch": {
        text: "Put the files into BepInEx\\Translation\\ru\\Text, replacing the old ones (14_new.txt holds the new lines), and press Alt+R in the game.",
        description: "Hint in the export dialog for the main format.",
    },
    "export.archive": {
        text: "Download the archive {name}",
        description: "Button downloading the whole export as a ZIP; name is the archive's file name.",
    },
    "save.hereShort": {
        text: "This text stands in {count} places — the translation changes everywhere.",
        description: "In the save dialog instead of the one-place choice, for a line that allows no one-place proposal (not a story line).",
    },
    "toggle.diff": {
        text: "Compare",
        description: "Switch above the story table: show each proposal as its changes against the text one step back.",
    },
    "toggle.diffHint": {
        text: "Added in green, removed in red.",
        description: "Hint under the Compare switch. One short line.",
    },
    "diff.vsGame": {
        text: "against the game's text",
        description: "Small note in a proposal's label line while Compare is on: its changes are shown against the Russian the game shows now.",
    },
    "diff.vsPrevious": {
        text: "against its previous version",
        description: "Small note in a proposal's label line while Compare is on: the proposal was rewritten, and its changes are shown against its previous text.",
    },
    "diff.gameChanged": {
        text: "The game's text changed on {date}",
        description: "Label over the comparison of the game's Russian with what it was before the last pack update (import). date is a formatted date.",
    },
    "diff.added": {
        text: "added",
        description: "Screen-reader word before a fragment the new version added.",
    },
    "diff.removed": {
        text: "removed",
        description: "Screen-reader word before a fragment the new version dropped.",
    },
    "action.history": {
        text: "History",
        description: "Button on a story line: opens the history of the line's translation.",
    },
    "lineHistory.title": {
        text: "History of the translation",
        description: "Title of the dialog with the history of one line's translation.",
    },
    "lineHistory.legend": {
        text: "Each text is compared with the one before it: added in green, removed in red.",
        description: "Explanation under the title of the line history dialog.",
    },
    "lineHistory.label": {
        text: "Changes, newest first",
        description: "Accessible name of the list in the line history dialog.",
    },
    "lineHistory.now": {
        text: "In the game now",
        description: "Label of the first row of the line history: the Russian the game shows now.",
    },
    "lineHistory.empty": {
        text: "Nothing has changed here yet.",
        description: "Shown in the line history dialog when the line has no proposals, decisions or pack changes.",
    },
    "lineHistory.game": {
        text: "The game's text changed (pack update)",
        description: "Line history entry: an import brought a new Russian from the pack.",
    },
    "lineHistory.proposed": {
        text: "{name} proposed",
        description: "Line history entry: a translator wrote a proposal. name is their nickname.",
    },
    "lineHistory.rewrote": {
        text: "{name} rewrote the proposal",
        description: "Line history entry: a translator rewrote their proposal. name is their nickname.",
    },
    "lineHistory.approved": {
        text: "{name} approved",
        description: "Line history entry: the lead approved this text. name is their nickname.",
    },
    "lineHistory.approvedGame": {
        text: "{name} checked the game's text and kept it",
        description: "Line history entry: the lead approved the game's Russian as it is. name is their nickname.",
    },
    "lineHistory.withdrew": {
        text: "{name} withdrew the approval",
        description: "Line history entry: the lead took an approval back. name is their nickname.",
    },
    "lineHistory.deleted": {
        text: "deleted",
        description: "Small mark in the line history: this proposal was deleted since.",
    },
    "rule.label": {
        text: '"Recommended" rule',
        description: "Label of the lead's select above the story table: which rule puts the recommended star on a line's variant.",
    },
    "rule.B": {
        text: "B — only where the game's text has a mark",
        description: "Option of the recommended rule: a star only on lines whose game text fails a check. The default.",
    },
    "rule.A": {
        text: "A — the best variant of every line",
        description: "Option of the recommended rule: a star on the best passing variant of every line.",
    },
    "rule.hint": {
        text: "One rule for the whole team.",
        description: "Hint next to the recommended rule select. One short line.",
    },
    "rule.failed": {
        text: "Couldn't change the rule.",
        description: "Toast when saving the recommended rule failed.",
    },
    "tabs.search": {
        text: "Search",
        description: "Tab of the translators' section: search every text of the game and edit it in the results.",
    },
    "search.label": {
        text: "Search every text",
        description: "Search tab of the translators' section: accessible name of the search box.",
    },
    "search.placeholder": {
        text: "English or Russian…",
        description: "Search tab of the translators' section: placeholder of the search box (it searches both).",
    },
    "search.clear": {
        text: "Clear",
        description: "Search tab of the translators' section: button that clears the search box.",
    },
    "search.whole": {
        text: "Whole word",
        description: "Search tab of the translators' section: checkbox, match whole words only.",
    },
    "search.section": {
        text: "Section",
        description: "Search tab of the translators' section: label of the section select.",
    },
    "search.status": {
        text: "State",
        description: "Search tab of the translators' section: label of the state select.",
    },
    "search.section.all": {
        text: "All",
        description: "Search tab of the translators' section: section select, every section.",
    },
    "search.section.story": {
        text: "Story",
        description: "Search tab of the translators' section: section select.",
    },
    "search.section.operators": {
        text: "Operators",
        description: "Search tab of the translators' section: section select.",
    },
    "search.section.enemies": {
        text: "Enemies",
        description: "Search tab of the translators' section: section select.",
    },
    "search.section.stages": {
        text: "Stages",
        description: "Search tab of the translators' section: section select.",
    },
    "search.section.items": {
        text: "Items",
        description: "Search tab of the translators' section: section select.",
    },
    "search.section.archives": {
        text: "Archives",
        description: "Search tab of the translators' section: section select (operator files, voice lines, outfit and module stories).",
    },
    "search.section.other": {
        text: "Interface and other",
        description: "Search tab of the translators' section: section select, the rest of the game's texts.",
    },
    "search.status.all": {
        text: "All",
        description: "Search tab of the translators' section: state select, any state.",
    },
    "search.status.unchecked": {
        text: "Not checked",
        description: "Search tab of the translators' section: state select, nobody proposed or approved anything.",
    },
    "search.status.proposed": {
        text: "Has proposals",
        description: "Search tab of the translators' section: state select.",
    },
    "search.status.approved": {
        text: "Approved",
        description: "Search tab of the translators' section: state select.",
    },
    "search.status.untranslated": {
        text: "Not translated",
        description: "Search tab of the translators' section: state select, the game shows no Russian.",
    },
    "search.found": {
        text: "Found: {count} · {millis} ms",
        description: "Search tab of the translators' section: how many texts match and how long the search took; count and millis come formatted.",
    },
    "search.prompt": {
        text: "Type a word, or choose a section or a state.",
        description: "Search tab of the translators' section: shown before anything is asked.",
    },
    "search.none": {
        text: "Nothing found.",
        description: "Search tab of the translators' section: no text matches.",
    },
    "search.more": {
        text: "Show more",
        description: "Search tab of the translators' section: button under the results that loads the next 50.",
    },
    "search.shown": {
        text: "Shown {shown} of {total}",
        description: "Search tab of the translators' section: next to Show more; numbers come formatted.",
    },
    "search.load": {
        text: "Couldn't search.",
        description: "Search tab of the translators' section: the search failed.",
    },
    "search.table": {
        text: "Found texts",
        description: "Search tab of the translators' section: accessible name of the results table.",
    },
    "search.where": {
        text: "Where",
        description: "Search tab of the translators' section: column header, where the text stands in the game.",
    },
    "search.alsoIn": {
        text: "{count} more",
        description: "Search tab of the translators' section: a button under the first place of a found text saying how many more places hold the same text; a click unfolds the list.",
    },
    "where.text": {
        text: "Game text · open in Search",
        description: "My edits: where a proposal for a text outside the story was made; links to the Search tab.",
    },
    "at.story": {
        text: "Story",
        description: "One part of where a text stands in the game data, as in «Operator › Amiya › Skill 2 › description»: the section, singular.",
    },
    "at.operators": {
        text: "Operator",
        description: "One part of where a text stands in the game data, as in «Operator › Amiya › Skill 2 › description»: the section, singular.",
    },
    "at.enemies": {
        text: "Enemy",
        description: "One part of where a text stands in the game data, as in «Operator › Amiya › Skill 2 › description»: the section, singular.",
    },
    "at.stages": {
        text: "Stage",
        description: "One part of where a text stands in the game data, as in «Operator › Amiya › Skill 2 › description»: the section, singular.",
    },
    "at.items": {
        text: "Item",
        description: "One part of where a text stands in the game data, as in «Operator › Amiya › Skill 2 › description»: the section, singular.",
    },
    "at.archives": {
        text: "Archives",
        description: "One part of where a text stands in the game data, as in «Operator › Amiya › Skill 2 › description»: the section, singular.",
    },
    "at.other": {
        text: "Interface and other",
        description: "One part of where a text stands in the game data, as in «Operator › Amiya › Skill 2 › description»: the section, singular.",
    },
    "at.part.talent": {
        text: "Talent",
        description: "One part of where a text stands in the game data, as in «Operator › Amiya › Skill 2 › description»: which object part; a number may follow.",
    },
    "at.part.skill": {
        text: "Skill",
        description: "One part of where a text stands in the game data, as in «Operator › Amiya › Skill 2 › description»: which object part; a number may follow.",
    },
    "at.part.module": {
        text: "Module",
        description: "One part of where a text stands in the game data, as in «Operator › Amiya › Skill 2 › description»: which object part; a number may follow.",
    },
    "at.part.base": {
        text: "Base skill",
        description: "One part of where a text stands in the game data, as in «Operator › Amiya › Skill 2 › description»: which object part; a number may follow.",
    },
    "at.part.skin": {
        text: "Outfit",
        description: "One part of where a text stands in the game data, as in «Operator › Amiya › Skill 2 › description»: which object part; a number may follow.",
    },
    "at.part.file": {
        text: "File",
        description: "One part of where a text stands in the game data, as in «Operator › Amiya › Skill 2 › description»: which object part; a number may follow.",
    },
    "at.part.voice": {
        text: "Voice line",
        description: "One part of where a text stands in the game data, as in «Operator › Amiya › Skill 2 › description»: which object part; a number may follow.",
    },
    "at.part.zone": {
        text: "Zone",
        description: "One part of where a text stands in the game data, as in «Operator › Amiya › Skill 2 › description»: which object part; a number may follow.",
    },
    "at.field.name": {
        text: "name",
        description: "One part of where a text stands in the game data, as in «Operator › Amiya › Skill 2 › description»: the field, lower case.",
    },
    "at.field.appellation": {
        text: "appellation",
        description: "One part of where a text stands in the game data, as in «Operator › Amiya › Skill 2 › description»: the field, lower case.",
    },
    "at.field.subclass": {
        text: "branch",
        description: "One part of where a text stands in the game data, as in «Operator › Amiya › Skill 2 › description»: the field, lower case.",
    },
    "at.field.tag": {
        text: "tag",
        description: "One part of where a text stands in the game data, as in «Operator › Amiya › Skill 2 › description»: the field, lower case.",
    },
    "at.field.trait": {
        text: "trait",
        description: "One part of where a text stands in the game data, as in «Operator › Amiya › Skill 2 › description»: the field, lower case.",
    },
    "at.field.description": {
        text: "description",
        description: "One part of where a text stands in the game data, as in «Operator › Amiya › Skill 2 › description»: the field, lower case.",
    },
    "at.field.potential": {
        text: "potential",
        description: "One part of where a text stands in the game data, as in «Operator › Amiya › Skill 2 › description»: the field, lower case.",
    },
    "at.field.mission": {
        text: "mission",
        description: "One part of where a text stands in the game data, as in «Operator › Amiya › Skill 2 › description»: the field, lower case.",
    },
    "at.field.effect": {
        text: "effect",
        description: "One part of where a text stands in the game data, as in «Operator › Amiya › Skill 2 › description»: the field, lower case.",
    },
    "at.field.story": {
        text: "story",
        description: "One part of where a text stands in the game data, as in «Operator › Amiya › Skill 2 › description»: the field, lower case.",
    },
    "at.field.group": {
        text: "series",
        description: "One part of where a text stands in the game data, as in «Operator › Amiya › Skill 2 › description»: the field, lower case.",
    },
    "at.field.usage": {
        text: "usage",
        description: "One part of where a text stands in the game data, as in «Operator › Amiya › Skill 2 › description»: the field, lower case.",
    },
    "at.field.dialog": {
        text: "quote",
        description: "One part of where a text stands in the game data, as in «Operator › Amiya › Skill 2 › description»: the field, lower case.",
    },
    "at.field.title": {
        text: "title",
        description: "One part of where a text stands in the game data, as in «Operator › Amiya › Skill 2 › description»: the field, lower case.",
    },
    "at.field.text": {
        text: "text",
        description: "One part of where a text stands in the game data, as in «Operator › Amiya › Skill 2 › description»: the field, lower case.",
    },
    "at.field.ability": {
        text: "ability",
        description: "One part of where a text stands in the game data, as in «Operator › Amiya › Skill 2 › description»: the field, lower case.",
    },
    "at.field.obtain": {
        text: "how to get",
        description: "One part of where a text stands in the game data, as in «Operator › Amiya › Skill 2 › description»: the field, lower case.",
    },
    "at.field.first": {
        text: "name, part 1",
        description: "One part of where a text stands in the game data, as in «Operator › Amiya › Skill 2 › description»: the field, lower case.",
    },
    "at.field.second": {
        text: "name, part 2",
        description: "One part of where a text stands in the game data, as in «Operator › Amiya › Skill 2 › description»: the field, lower case.",
    },
    "at.field.third": {
        text: "name, part 3",
        description: "One part of where a text stands in the game data, as in «Operator › Amiya › Skill 2 › description»: the field, lower case.",
    },
    "at.field.locked": {
        text: "when locked",
        description: "One part of where a text stands in the game data, as in «Operator › Amiya › Skill 2 › description»: the field, lower case.",
    },
    "at.table.rogue": {
        text: "Integrated Strategies",
        description: "A group of the game's tables in «Interface and other» of the translators' search: where a found text stands, in words.",
    },
    "at.table.events": {
        text: "Events",
        description: "A group of the game's tables in «Interface and other» of the translators' search: where a found text stands, in words.",
    },
    "at.table.base": {
        text: "Base",
        description: "A group of the game's tables in «Interface and other» of the translators' search: where a found text stands, in words.",
    },
    "at.table.medals": {
        text: "Medals",
        description: "A group of the game's tables in «Interface and other» of the translators' search: where a found text stands, in words.",
    },
    "at.table.sandbox": {
        text: "Reclamation Algorithm",
        description: "A group of the game's tables in «Interface and other» of the translators' search: where a found text stands, in words.",
    },
    "at.table.ui": {
        text: "Interface labels",
        description: "A group of the game's tables in «Interface and other» of the translators' search: where a found text stands, in words.",
    },
    "at.table.stages": {
        text: "Stages (other)",
        description: "A group of the game's tables in «Interface and other» of the translators' search: where a found text stands, in words.",
    },
    "at.table.files": {
        text: "Operator files (other)",
        description: "A group of the game's tables in «Interface and other» of the translators' search: where a found text stands, in words.",
    },
    "at.table.operators": {
        text: "Operators (other)",
        description: "A group of the game's tables in «Interface and other» of the translators' search: where a found text stands, in words.",
    },
    "at.table.voice": {
        text: "Operator voice lines (other)",
        description: "A group of the game's tables in «Interface and other» of the translators' search: where a found text stands, in words.",
    },
    "at.table.display": {
        text: "Profile decorations",
        description: "A group of the game's tables in «Interface and other» of the translators' search: where a found text stands, in words.",
    },
    "at.table.review": {
        text: "Story archive",
        description: "A group of the game's tables in «Interface and other» of the translators' search: where a found text stands, in words.",
    },
    "at.table.shop": {
        text: "Store",
        description: "A group of the game's tables in «Interface and other» of the translators' search: where a found text stands, in words.",
    },
    "at.table.missions": {
        text: "Missions",
        description: "A group of the game's tables in «Interface and other» of the translators' search: where a found text stands, in words.",
    },
    "at.table.gacha": {
        text: "Headhunting",
        description: "A group of the game's tables in «Interface and other» of the translators' search: where a found text stands, in words.",
    },
    "at.table.tower": {
        text: "Stationary Security Service",
        description: "A group of the game's tables in «Interface and other» of the translators' search: where a found text stands, in words.",
    },
    "at.table.skins": {
        text: "Outfits (other)",
        description: "A group of the game's tables in «Interface and other» of the translators' search: where a found text stands, in words.",
    },
    "at.table.charms": {
        text: "Charms",
        description: "A group of the game's tables in «Interface and other» of the translators' search: where a found text stands, in words.",
    },
    "at.table.audio": {
        text: "Music",
        description: "A group of the game's tables in «Interface and other» of the translators' search: where a found text stands, in words.",
    },
    "at.table.crisis": {
        text: "Contingency Contract",
        description: "A group of the game's tables in «Interface and other» of the translators' search: where a found text stands, in words.",
    },
    "at.table.tips": {
        text: "Loading tips",
        description: "A group of the game's tables in «Interface and other» of the translators' search: where a found text stands, in words.",
    },
    "at.table.modules": {
        text: "Modules (other)",
        description: "A group of the game's tables in «Interface and other» of the translators' search: where a found text stands, in words.",
    },
    "at.table.enemies": {
        text: "Enemies (other)",
        description: "A group of the game's tables in «Interface and other» of the translators' search: where a found text stands, in words.",
    },
    "at.table.teams": {
        text: "Factions",
        description: "A group of the game's tables in «Interface and other» of the translators' search: where a found text stands, in words.",
    },
    "at.table.skills": {
        text: "Skills (other)",
        description: "A group of the game's tables in «Interface and other» of the translators' search: where a found text stands, in words.",
    },
    "at.table.clues": {
        text: "Clues",
        description: "A group of the game's tables in «Interface and other» of the translators' search: where a found text stands, in words.",
    },
    "at.table.campaign": {
        text: "Annihilation",
        description: "A group of the game's tables in «Interface and other» of the translators' search: where a found text stands, in words.",
    },
    "at.table.items": {
        text: "Items (other)",
        description: "A group of the game's tables in «Interface and other» of the translators' search: where a found text stands, in words.",
    },
    "at.file.ui": {
        text: "Interface dictionary",
        description: "Where a found text tied to no place of the game data is kept, in words (translators' search); the file name shows on hover.",
    },
    "at.file.story": {
        text: "Story dictionary",
        description: "Where a found text tied to no place of the game data is kept, in words (translators' search); the file name shows on hover.",
    },
    "at.file.names": {
        text: "Names dictionary",
        description: "Where a found text tied to no place of the game data is kept, in words (translators' search); the file name shows on hover.",
    },
    "at.file.new": {
        text: "New entries",
        description: "Where a found text tied to no place of the game data is kept, in words (translators' search); the file name shows on hover.",
    },
    "at.file.auto": {
        text: "Automatic translation",
        description: "Where a found text tied to no place of the game data is kept, in words (translators' search); the file name shows on hover.",
    },
    "at.file.plugin": {
        text: "Plugin catalog",
        description: "Where a found text tied to no place of the game data is kept, in words (translators' search); the file name shows on hover.",
    },
    "search.fewer": {
        text: "Collapse",
        description: "Button that folds the list of a found text's places back to the first one (translators' search).",
    },
    "tabs.sections": {
        text: "Game sections",
        description: "Tab of the translators' section: the game's texts by section (operators, enemies, stages, items, interface), an object at a time.",
    },
    "sections.section": {
        text: "Section",
        description: "Label of the picker of a game section (operators, enemies, stages, items, interface and other) on the Game sections tab.",
    },
    "sections.object": {
        text: "Object",
        description: "Label of the picker of one object of the chosen game section (an operator, an enemy, a stage, an item).",
    },
    "sections.objectPlaceholder": {
        text: "Search: {count, plural, one {# object} other {# objects}}",
        description: "Placeholder of the object picker on the Game sections tab; it searches the Russian and English names.",
    },
    "sections.objectEmpty": {
        text: "No such objects.",
        description: "Shown in the object picker of the Game sections tab when nothing matches the typed name.",
    },
    "sections.choose": {
        text: "Choose an object.",
        description: "Shown on the Game sections tab until an object is chosen.",
    },
    "sections.load.objects": {
        text: "Couldn't load the section's objects.",
        description: "Load failure of the object list on the Game sections tab.",
    },
    "sections.load.lines": {
        text: "Couldn't load the object's texts.",
        description: "Load failure of one object's texts on the Game sections tab.",
    },
    "sections.table": {
        text: "Texts: {object}",
        description: "Accessible name of the table of one game object's texts.",
    },
    "sections.field": {
        text: "Field",
        description: "Column header of the table of a game object's texts: what of the object the text is (name, skill 2 description…).",
    },
    "sections.code": {
        text: "Code",
        description: "Row of a stage's card on the Game sections tab: the stage's code (0-1), which is shown and never translated.",
    },
    "sections.kind.token": {
        text: "summon",
        description: "Under an object's name in the operator picker of the Game sections tab: the object is a summoned unit, not an operator.",
    },
    "sections.kind.trap": {
        text: "device",
        description: "Under an object's name in the operator picker of the Game sections tab: the object is a deployable device or a stage element, not an operator.",
    },
    "sections.progress": {
        text: "{checked} checked · {waiting} waiting",
        description: "Short progress of one game object in its picker: the share of its texts approved and how many wait for a decision.",
    },
    "sections.summary": {
        text: "{count, plural, one {# text} other {# texts}}, {ru} with Russian",
        description: "Above the table of a game object's texts: how many texts it has and how many of them have Russian in the game.",
    },
    "sections.progressLong": {
        text: "Checked {checked} · waiting for a decision {waiting}",
        description: "Next to the progress bar above a game object's texts.",
    },
    "sections.progressLabel": {
        text: "Progress of the object",
        description: "Accessible name of the progress bar above a game object's texts.",
    },
    "at.field.class": {
        text: "class",
        description: "What of an operator a text is, in the list of a text's places: the operator's class (Guard, Caster…).",
    },
    "at.field.who": {
        text: "name",
        description: "What of an operator or an enemy a text is, in the list of a text's places and in the object's card: its name (an item or a stage has a «title» instead).",
    },
    "editor.insertToken": {
        text: "Insert {token}",
        description: "Tooltip of a button under the editor that inserts one substitution of the English original ({0}, {atk:0%}) at the cursor.",
    },
    "editor.removeToken": {
        text: "Remove",
        description: "Accessible name of the small cross on a substitution chip in the editor: removes the chip.",
    },
    "at.note.elite": {
        text: "Elite {n}",
        description: "Part of the small line under a card's field name telling one text from the others at the same field: the operator's elite phase it comes at ({n} is 0, 1 or 2).",
    },
    "at.note.level": {
        text: "Lv. {n}",
        description: "Part of the small line under a card's field name telling one text from the others at the same field: the operator's level it comes at.",
    },
    "at.note.potential": {
        text: "potential {n}",
        description: "Part of the small line under a card's field name telling one text from the others at the same field: the potential it needs ({n} is 2 to 6).",
    },
    "at.note.skillLevel": {
        text: "skill level {n}",
        description: "Part of the small line under a card's field name telling one text from the others at the same field: the skill's level, or a range of levels (1–7).",
    },
    "at.note.mastery": {
        text: "mastery {n}",
        description: "Part of the small line under a card's field name telling one text from the others at the same field: the skill's mastery level, or a range (1–3).",
    },
    "at.note.moduleStage": {
        text: "module stage {n}",
        description: "Part of the small line under a card's field name telling one text from the others at the same field: the module's stage, or a range (1–3).",
    },
    "at.note.number": {
        text: "No. {n}",
        description: "Part of the small line under a card's field name telling one text from the others at the same field: its number in a list (the second mission of a module, the third tag).",
    },
    "at.note.challenge": {
        text: "Challenge Mode",
        description: "Part of the small line under a card's field name telling one text from the others at the same field: the stage's challenge mode.",
    },
    "at.note.easy": {
        text: "Story Environment",
        description: "Part of the small line under a card's field name telling one text from the others at the same field: the stage in the story environment of a later main chapter.",
    },
    "at.note.normal": {
        text: "Standard Environment",
        description: "Part of the small line under a card's field name telling one text from the others at the same field: the stage in the standard environment of a later main chapter.",
    },
    "at.note.tough": {
        text: "Adverse Environment",
        description: "Part of the small line under a card's field name telling one text from the others at the same field: the stage in the adverse environment of a later main chapter.",
    },
    "sections.same": {
        text: "{count, plural, one {# record} other {# identical records}} of the game",
        description: "Under an object's name in the picker of the Game sections tab: the game data keeps several records with this name and the very same texts; they are one card.",
    },
    "at.part.record": {
        text: "Operator record",
        description: "A part of an operator's archives in a place («Operator record 1 › description»): one of the operator's records (short personal stories unlocked with trust).",
    },
    "at.note.voiceSet": {
        text: "voice set {n}",
        description: "Part of the small line under a card's field name telling one text from the others at the same field: which set of voice lines it belongs to (the operator's own is 1; another form's or a dialect's follow).",
    },
    "long.show": {
        text: "Show all",
        description: "Small button in a table row whose text is long and folded to a few lines: unfolds the whole row.",
    },
    "long.hide": {
        text: "Fold",
        description: "Small button in a table row whose long text is unfolded: folds it back to a few lines.",
    },
    "sections.readStory": {
        text: "Read in Story",
        description: "Link under an operator record's introduction in the archives card: opens the record's story in the Story tab.",
    },
    "search.objects": {
        text: "Objects of the game named so",
        description: "Accessible name of the short list above the search results: the operators, enemies, items and stages whose name matches the query.",
    },
    "search.openCard": {
        text: "Open the card",
        description: "Link beside an object above the search results: opens the object's card on the Game sections tab.",
    },
    "selection.label": {
        text: "Selected text",
        description: "Accessible name of the small menu that appears at a word or phrase selected in a table of the translators' section.",
    },
    "selection.find": {
        text: "Find everywhere",
        description: "Button of the selected text's menu: opens the Search tab on the selected word or phrase.",
    },
    "selection.glossary": {
        text: "Find in the glossary",
        description: "Button of the selected text's menu: opens the Glossary tab filtered by the selected word.",
    },
    "selection.suggest": {
        text: "Suggest for the glossary",
        description: "Button of the selected text's menu: opens the dialog that sends the selected term to the lead as a suggestion for the glossary.",
    },
    "suggest.title": {
        text: "Suggest for the glossary",
        description: "Title of the dialog that suggests a term for the team's glossary.",
    },
    "suggest.description": {
        text: "The lead will see it in the Glossary tab. The team's sheet is not changed from here.",
        description: "Line under the title of the suggest-for-the-glossary dialog.",
    },
    "suggest.en": {
        text: "English",
        description: "Label of the English term field in the suggest-for-the-glossary dialog.",
    },
    "suggest.ru": {
        text: "Russian",
        description: "Label of the Russian translation field in the suggest-for-the-glossary dialog.",
    },
    "suggest.note": {
        text: "Note (optional)",
        description: "Label of the free-text note in the suggest-for-the-glossary dialog.",
    },
    "suggest.from": {
        text: "Noticed in: {en}",
        description: "Small line in the suggest dialog: the English of the line the term was selected in.",
    },
    "suggest.send": {
        text: "Send",
        description: "Button that sends a glossary suggestion to the lead.",
    },
    "suggest.sent": {
        text: "Sent to the lead.",
        description: "Toast after a glossary suggestion was saved.",
    },
    "suggest.failed": {
        text: "Couldn't save the suggestion.",
        description: "Toast when saving, deciding or deleting a glossary suggestion failed.",
    },
    "suggest.list": {
        text: "Suggested for the glossary: {count} · waiting {waiting}",
        description: "Button on the Glossary tab unfolding the list of terms members suggested; {waiting} of them have no decision yet.",
    },
    "suggest.listHint": {
        text: "The lead accepts or declines; an accepted term is entered into the sheet by hand.",
        description: "Line above the list of suggested glossary terms.",
    },
    "suggest.status.open": {
        text: "waiting",
        description: "Badge on a suggested glossary term: no decision yet.",
    },
    "suggest.status.accepted": {
        text: "accepted",
        description: "Badge on a suggested glossary term: the lead accepted it.",
    },
    "suggest.status.declined": {
        text: "declined",
        description: "Badge on a suggested glossary term: the lead declined it.",
    },
    "suggest.by": {
        text: "{name}, {date}",
        description: "Who suggested a glossary term and when.",
    },
    "suggest.decidedBy": {
        text: "decided by {name}",
        description: "Who decided on a suggested glossary term.",
    },
    "suggest.accept": {
        text: "Accept",
        description: "The lead's button on a suggested glossary term.",
    },
    "suggest.decline": {
        text: "Decline",
        description: "The lead's button on a suggested glossary term.",
    },
    "suggest.reopen": {
        text: "Reopen",
        description: "The lead's button on a decided glossary suggestion: back to waiting.",
    },
    "suggest.delete": {
        text: "Delete",
        description: "Button deleting a glossary suggestion (its author or the lead).",
    },
} satisfies MessageMap;

export const { keys } = defineMessages({ namespace, messages });
