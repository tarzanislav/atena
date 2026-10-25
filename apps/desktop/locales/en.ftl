# ATENA – interface texts in English (the pivot language for translators).
# Slovak (sk.ftl) is the reference; every message here has the same identifier.
# Translated by a machine: a draft until a native speaker has reviewed it.

language-name = English

## The app

app-claim = Verified information about education
app-version = Version { $version } · open-source software
translation-draft = translation – suggestion
yes = yes
no = no
action-close = Close
action-cancel = Cancel
action-open-original = Open the original

## Start

startup-starting = Starting ATENA…
startup-first-time = At the first start ATENA prepares its own database on this computer and loads the register of sources. It takes about half a minute; next time it is quicker.
startup-failed-hint = Close ATENA and open it again. If the error comes back, the details are in the log in the app's folder (logs/atena.log).

## Navigation

nav-label = Main menu
nav-overview = Overview
nav-search = Search
nav-tools = Tools
nav-sources = Sources
nav-proposals = Proposals
nav-crawl = Crawling
nav-settings = Settings

## Overview

overview-title = Overview
overview-empty-title = Nothing has been collected yet
overview-empty-text = Start crawling. ATENA visits the sources of the register as a polite visitor: it keeps their rules, never hurries and takes only what the law and the licence allow.
overview-empty-action = Go to crawling
overview-documents =
    { $count ->
        [one] document
       *[other] documents
    }
overview-versions =
    { $count ->
        [one] { $count } version
       *[other] { $count } versions
    }
overview-searchable = searchable documents
overview-sources = sources are crawled (of all in the register)
overview-proposals =
    { $count ->
        [one] proposal awaits a decision
       *[other] proposals await a decision
    }
overview-last-check = Last check: { $time } · in the last 24 hours { $checks ->
        [one] { $checks } check
       *[other] { $checks } checks
    }
overview-never-checked = The sources have not been checked yet.
overview-newest = Newest
overview-search-all = Search everything
overview-sections = Categories
overview-in-scope =
    { $total ->
        [one] { $total } document
       *[other] { $total } documents
    } in this selection
overview-new-in-scope = { $new } new in { $days } days
overview-counts-legend = Next to the categories: new in the last { $days } days / all
overview-counts-title = { $new } new in the last { $days } days, { $total } in all
overview-sources-line = { $crawled } of { $sources } sources are crawled

## Slovak or foreign data, trust in the source

scope-label = Which data to show
scope-sk = Slovakia
scope-abroad = Abroad
scope-languages = Language of foreign data
scope-all-languages = All languages
trust-official = Official source
trust-expert = Expert source
trust-other = Other source – check it
tools-title = Tools and dictionaries
tools-intro = Dictionaries, corpora and applications ATENA does not crawl – you search them directly. They open in your browser.
tools-open = Open the tool
tools-none = There are no tools for this data yet.

## Search

search-title = Search
search-placeholder = Methods, regulations, materials… in any language
search-action = Search
search-hint = Type what you are looking for, in any language. ATENA also searches other languages through the concepts it recognises in your words.
search-hint-syntax = Put a phrase in quotes ("reading literacy"); exclude a word with a minus (-holidays).
search-filters = Narrowed to:
search-filter-remove = Remove this filter
search-narrow = Narrow the results
search-found =
    { $count ->
        [0] No document was found.
        [one] { $count } document found.
       *[other] { $count } documents found.
    }
search-some-words = No document has all the words – these have some of them.
search-concepts = Also searched for the concepts:
search-nothing = Try other words or remove a filter.
search-previous = Previous
search-next = Next
search-page = Page { $page } of { $pages }
search-only-official = Official sources only
search-tools = Tools for the words you searched
hit-no-text = The text of this source is not shown in ATENA – its licence does not allow it. Open the original.

facet-section = Section
facet-school-type = Type of school
facet-resource-type = Type of content
facet-topic = Topic
facet-country = Country
facet-authority = Authority of the source
facet-language = Language
facet-source = Source
facet-trust = Trust in the source

## Document card

document-title = Document
document-untitled = (untitled)
document-not-shown = ATENA does not show this document: it was removed, its publisher does not want it to be found, or its source may not be crawled.
document-published = published { $date }
document-labels-are-suggestions = Sections and topics are ATENA's suggestion until a person reviews them.
document-text = Text
document-text-not-shown = The full text is not shown in ATENA – the licence of the source ({ $licence }) does not allow it. Read it in the original.
document-versions =
    { $count ->
        [one] { $count } version
       *[other] { $count } versions
    }
version-number = Version
version-captured = Captured
version-change = Change
version-paragraphs = (+{ $added } / −{ $removed } paragraphs)
version-numbers-changed = a number or date changed

change-initial = first capture
change-minor = minor change
change-substantive = substantive change
change-changed = the text changed (the previous one is not kept)

## Sources

sources-title = Sources
sources-intro = The sources of the register and confirmed proposals. For each you see what the legality principle allows: whether it may be crawled, whether its text may be kept and whether it may be shown in full.
sources-sync = Sync the shared register
sources-sync-explain = ATENA downloads the current shared register of sources from the ATENA repository on GitHub (the file data/sources.yaml, over an encrypted https connection only) and brings the database in line with it. What the legality principle no longer allows is removed.
sources-sync-privacy = Only an ordinary request for the file is sent; nothing about you or your proposals.
sources-sync-confirm = Download and sync
sources-synced = The register is in line: { $sources } sources, { $added } new addresses. Documents removed: { $purged }, versions reduced to metadata: { $restricted }.
sources-show-all = All
sources-show-crawled = Crawled
sources-show-not-crawled = Not crawled
sources-show-imported = From proposals
sources-filter = Filter by name, address or country
sources-column-source = Source
sources-column-country = Country
sources-column-rights = Licence and terms
sources-column-crawl = Crawl
sources-column-text = Text
sources-column-show = Show
sources-column-documents = Documents
sources-checked = checked { $date }
sources-tool = Tool (not crawled)
sources-site-wide = Whole site
sources-none = No source matches.

allowed-crawl = may be crawled
allowed-text = its text may be kept
allowed-show = its full text may be shown

licence-copyrighted = protected by copyright
licence-official = official text (free of copyright)

terms-allowed = terms allow it
terms-forbidden = terms forbid it
terms-reserved = reserved (text and data mining)
terms-unknown = terms not verified

state-proposed = proposed
state-confirmed = confirmed
state-rejected = rejected
state-merged = taken over by the register

restriction-no-profile = ATENA has no legal profile for the source's country
restriction-terms-forbid = the terms of use forbid automated access
restriction-terms-not-verified = protected content: nobody has verified the terms of use yet
restriction-research-only = protected content: the source's country allows mining only for non-commercial research
restriction-no-mining-exception = protected content: the source's country has no exception for text and data mining
restriction-mining-reserved = the publisher reserved text and data mining
restriction-privacy-law = the data protection law of the source's country does not allow keeping texts abroad

## Proposals

proposals-title = Proposed sources
proposals-intro = Propose pages ATENA should follow. It assesses every address first; crawling starts only when a person confirms the proposal.
proposals-tab-add = Add addresses
proposals-tab-waiting = Awaiting a decision
proposals-tab-decided = Decided
proposals-tab-share = Hand to the editor
proposals-tab-receive = Receive a list

import-addresses = Addresses or DOIs, one per line
import-placeholder = https://www.school.org/news/
import-format = After an address you may add the code of the country whose law applies (for example "https://www.skola.cz/ CZ"). At most 500 addresses at a time.
import-country = Country for addresses without a code
import-country-auto = from the address
import-how = ATENA opens every address as a polite visitor: it reads robots.txt, the licence statement and the terms of use and proposes what the law allows. Nothing is crawled until you confirm. Pages about people are not stored even as proposals.
import-action =
    { $count ->
        [0] Assess
        [one] Assess { $count } address
       *[other] Assess { $count } addresses
    }
import-busy = Assessing… with many addresses this may take a few minutes.
import-done =
    { $count ->
        [one] { $count } address assessed.
       *[other] { $count } addresses assessed.
    }
import-kept-for-sharing =
    { $count ->
        [one] { $count } address waits to be handed to the editor.
       *[other] { $count } addresses wait to be handed to the editor.
    }
import-column-line = Line
import-column-address = Address
import-column-result = Result
import-column-proposal = Proposal

outcome-new = new proposal
outcome-reassessed = proposal assessed again
outcome-already-proposed = already proposed
outcome-already-confirmed = already confirmed
outcome-already-rejected = already rejected
outcome-already-merged = already in the register
outcome-in-register = already in the register ({ $register })
outcome-blocked = blocked after an erasure request
outcome-personal-page = a page about people – not stored

problem-robots = robots.txt does not allow ATENA to read the page
problem-robots-unreachable = robots.txt could not be read, so nothing is read for now
problem-blocked = this address is not allowed (private network, unusual port…)
problem-status = the server answered with an error
problem-unreachable = the server could not be reached
problem-retry-later = the server asked us to come back later
problem-unsupported-type = ATENA does not read this kind of content
problem-too-large = the content is larger than ATENA reads
problem-dangerous = it serves a program or a document with active content – ATENA does not draw on such sources
problem-too-demanding = the content would need more memory or time than any real document – ATENA does not draw on it

reason-cannot-read = the address could not be read
reason-personal-page = a page about people
reason-automation-ban = the terms forbid automated access
reason-personal-use-only = the terms allow personal use only
reason-mining-reserved = the terms reserve text mining or AI
reason-tdm-reserved = TDMRep reserves text and data mining
reason-ai-crawlers-refused = robots.txt refuses crawlers for AI
reason-open-licence = open licence
reason-licence-not-open = the licence is not open or contradicts itself
reason-non-commercial = non-commercial use only
reason-rights-reserved = all rights reserved
reason-terms-read = terms read
reason-no-terms-found = no terms found
reason-terms-unreadable = the terms could not be read
reason-unknown-country = the country must be named
reason-dangerous-content = dangerous content

rejection-not-an-address = neither an address nor a DOI
rejection-unsupported-scheme = only http and https addresses
rejection-credentials = the address contains a name or password
rejection-duplicate = repeated
rejection-too-many = too many addresses at once
rejection-too-long = the line is too long

proposals-none-waiting = No proposal awaits a decision.
proposals-none-decided = You have not decided on any proposal yet.
proposal-shared = from another installation
proposal-confirm = Confirm
proposal-reject = Reject
proposal-automatic = ATENA's proposal ({ $date }): { $licence } · { $terms } · country { $country }
proposal-decided = Decision: { $licence } · { $terms } · country { $country }
proposal-would-allow = Confirming would allow:
proposal-finding = Rights record (in English, as it is kept)
proposal-rights-page = Open the page with the licence or terms
proposal-confirm-title = Confirm "{ $name }"
proposal-reject-title = Reject "{ $name }"
proposal-confirmed-crawled = Confirmed. The source will be crawled at the next crawl.
proposal-confirmed-not-crawled = Confirmed. The legality principle, however, does not allow crawling this source.
proposal-rejected = Rejected. Everything collected from this source has been removed.

decision-licence = Licence
decision-terms = Terms of use
decision-country = Country whose law applies
decision-reason-confirm = Why – what you read and when (required)
decision-reason-reject = Why you reject it (required)
decision-by = Decided by: { $name } (the name stays on this computer only)

share-explain = The list holds only addresses and country codes – nothing about you. ATENA sends it nowhere: copy it and hand it to the ATENA editor as you choose (for example by e-mail). Then mark it as handed over.
share-nothing = Nothing waits to be handed over.
share-list =
    { $count ->
        [one] A list with { $count } address
       *[other] A list with { $count } addresses
    }
share-copy = Copy
share-copied = Copied
share-mark-handed-over = Mark as handed over
share-handed-over = The list is marked as handed over.

receive-explain = For the editor: paste a list a user of another installation sent you. ATENA assesses every address itself – a received assessment would prove nothing – and stores them as proposals.
receive-list = The list (starts with the line ATENA-NAVRHY 1)
receive-action = Read the list
receive-not-a-list = This is not an ATENA list: the first line must be ATENA-NAVRHY 1.
receive-read-before = This list was read before ({ $list }).

## Crawling

crawl-title = Crawling
crawl-intro = ATENA visits the sources of the register and confirmed proposals, stores new and changed documents and makes them searchable. Crawling runs only when you start it.
crawl-state-running = Crawling
crawl-state-stopping = Stopping – the checks under way are finishing
crawl-state-stopped = Crawling is stopped
crawl-running = crawling
crawl-start = Start crawling
crawl-stop = Stop
crawl-started = started { $time }
crawl-checks = checks
crawl-hosts = servers visited
crawl-due = addresses due for a check
crawl-store-errors = database errors
crawl-outcomes = Results of the checks
crawl-rules-title = How ATENA crawls
crawl-rule-robots = It keeps robots.txt and the publisher's rules (TDMRep, noindex, nosnippet).
crawl-rule-pace = It visits every server slowly, one request after another, and waits when a server asks for rest.
crawl-rule-open = It draws only on open sources and keeps only what the law and the licence allow.
crawl-rule-privacy = It goes as an anonymous visitor: no cookies, no scripts, no images. It sends nothing about you.
crawl-rule-safety = It refuses programs and documents with active content, parses documents in a limited process and keeps no original files.
crawl-concurrency =
    { $count ->
        [one] It works on { $count } server at a time (you can change this in the settings).
       *[other] It works on { $count } servers at a time (you can change this in the settings).
    }

check-new = new document
check-changed = changed document
check-unchanged = unchanged
check-not-modified = unchanged (says the server)
check-feed = RSS/Atom feed read
check-sitemap = sitemap read
check-robots = robots.txt does not allow it
check-robots-unreachable = robots.txt unreachable
check-blocked = address not allowed
check-http-error = server error
check-gone = the document no longer exists
check-network = network unreachable
check-timeout = timed out
check-redirects = too many redirects
check-retry-later = the server asks to wait
check-too-large = too large
check-unsupported = unsupported kind of content
check-dangerous = dangerous content refused
check-too-demanding = too demanding content refused
check-extract-error = the content could not be processed
check-feed-error = broken RSS/Atom feed
check-sitemap-error = broken sitemap
check-moved-elsewhere = moved to another site
check-erased = erased on request
check-not-permitted = the legality principle does not allow it
check-invalid-url = invalid address
check-store-error = database error

## Settings

settings-title = Settings
settings-saved = Saved.
settings-appearance = Language and appearance
settings-language = Language of the interface
settings-theme = Appearance
settings-theme-system = as the system
settings-theme-light = light
settings-theme-dark = dark
settings-decisions = Decisions on proposals
settings-editor = Your name or nickname
settings-editor-privacy = It is recorded with your decisions and stays on this computer only. The shared register names no person.
settings-share = Keep proposed addresses for the editor
settings-share-explain = ATENA sends nothing by itself; the kept addresses are in Proposals → Hand to the editor.
settings-crawler = Crawling
settings-concurrency = Servers at a time (1–16)
settings-concurrency-explain = ATENA always sends each server one request at a time; this number says how many different servers it visits at once.

erasure-title = Erasure request
erasure-explain = When someone asks for erasure (GDPR Art. 17), ATENA removes everything about the address and never visits it again.
erasure-address = Address
erasure-prefix = also every address that starts with it
erasure-reason = Reason (for example: request of…)
erasure-action = Erase and block
erasure-done = Erased: { $documents } documents and { $versions } versions. The address is blocked.

about-title = About ATENA
about-version = ATENA { $version } – open-source software under the Apache 2.0 licence, free for private and commercial use.
about-connections = ATENA sends nothing about its users to anyone. It connects to nothing by itself; it goes online only when you start crawling, an assessment of addresses or a sync of the register, and then as an anonymous visitor. It uses no third-party services.
about-pillars = The four pillars of ATENA
pillar-1 = Seriousness, openness and the effort to move every person's knowledge forward.
pillar-2 = Anonymity and respect for privacy – ours and others'.
pillar-3 = Safety and protection against dangerous people, robots, code and processes.
pillar-4 = Respect for the law: no law may be broken, anywhere in the world.
about-licence = The licences of all parts are in the app's folder "notices".
about-repository = Source code on GitHub

## Errors

error-detail = Details
error-already-running = ATENA is already running. Work in the open window.
error-crawl = Crawling could not be prepared.
error-database = Database error.
error-database-start = The database could not be started.
error-database-version = The database was made by another version of PostgreSQL than the one this ATENA brings.
error-decision-country = ATENA has no legal profile for this country.
error-decision-incomplete = A decision needs a name and a reason.
error-decision-licence = The register does not know this licence.
error-decision-official = Official texts of this country are not free; choose its open licence.
error-decision-terms = The register does not know these terms.
error-editor-missing = A decision needs your name – enter it in the settings.
error-index = Search index error.
error-index-foreign = The index folder holds something other than an ATENA index.
error-input-too-long = The text is too long.
error-internal = An unexpected error occurred.
error-io = A file or folder could not be read or written.
error-not-a-list = This is not an ATENA list.
error-not-found = Not found.
error-not-ready = ATENA is still starting.
error-reason-missing = Please give a reason.
error-search = The search failed.
error-search-empty = Type what you are looking for, or choose a filter.
error-search-filter = ATENA does not know this filter.
error-settings = The settings could not be saved.
error-shared-register-invalid = The shared register is not valid; nothing has changed.
error-shared-register-unreachable = The shared register could not be downloaded. Check the internet connection.
error-url = This is not a valid web address.
