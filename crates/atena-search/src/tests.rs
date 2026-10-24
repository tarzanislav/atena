//! The index in memory: languages, concepts, filters, ranking and the legal limits.

use atena_core::{Taxonomy, suggest};
use atena_store::IndexRow;

use crate::document::tests::row;
use crate::{Results, SearchError, SearchIndex, SearchRequest, searchable};

fn taxonomy() -> Taxonomy {
    Taxonomy::bundled().unwrap()
}

fn doc(id: i64, language: &str, title: &str, body: &str) -> IndexRow {
    IndexRow {
        document_id: id,
        source_id: format!("S{id}"),
        title: Some(title.into()),
        description: None,
        body: Some(body.into()),
        language: Some(language.into()),
        labels: Vec::new(),
        url: format!("https://example.sk/{id}"),
        ..row("official", "allowed")
    }
}

fn index(rows: &[IndexRow]) -> SearchIndex {
    let index = SearchIndex::in_memory().unwrap();
    index
        .apply(rows.iter().map(|r| (r.document_id, searchable(r))))
        .unwrap();
    index
}

fn search(index: &SearchIndex, text: &str, filters: &[&str]) -> Results {
    index
        .search(
            &taxonomy(),
            &SearchRequest {
                text: text.into(),
                language: None,
                filters: filters.iter().map(|f| f.to_string()).collect(),
                limit: 10,
                offset: 0,
            },
        )
        .unwrap()
}

fn ids(results: &Results) -> Vec<i64> {
    results.hits.iter().map(|h| h.document_id).collect()
}

#[test]
fn inflected_words_without_diacritics_are_found() {
    let i = index(&[
        doc(
            1,
            "sk",
            "Hodnotenie žiakov",
            "Čitateľská gramotnosť v základných školách.",
        ),
        doc(
            2,
            "cs",
            "Hodnocení žáků",
            "Čtenářská gramotnost na základních školách.",
        ),
        doc(3, "en", "Assessment", "Reading in primary schools."),
    ]);
    assert_eq!(ids(&search(&i, "citatelskej gramotnosti", &[])), [1]);
    // Slovak and Czech forms of the same word meet in both languages
    let mut both = ids(&search(&i, "školy", &[]));
    both.sort_unstable();
    assert_eq!(both, [1, 2]);
    assert_eq!(ids(&search(&i, "školy", &["language/sk"])), [1]);
    assert_eq!(ids(&search(&i, "školami", &["language/cs"])), [2]);
    assert_eq!(ids(&search(&i, "school", &[])), [3]);
    assert_eq!(i.len(), 3);
}

#[test]
fn concepts_carry_a_search_across_languages() {
    let t = taxonomy();
    let words = "čitateľská gramotnosť";
    let concept = suggest(&t, words, "", Some("sk"), 0.45)
        .into_iter()
        .next()
        .expect("the taxonomy knows reading literacy");
    let mut english = doc(
        2,
        "en",
        "PIRLS results",
        "Results of the international study.",
    );
    english.labels = vec![(concept.scheme.clone(), concept.concept.clone())];
    let i = index(&[
        doc(1, "sk", "Čitateľská gramotnosť", "Výsledky štúdie PIRLS."),
        english,
        doc(3, "en", "Mathematics", "Results of another study."),
    ]);
    let r = search(&i, words, &[]);
    assert_eq!(
        ids(&r),
        [1, 2],
        "the Slovak words first, then the labelled English document"
    );
    assert!(r.concepts.iter().any(|c| c.concept == concept.concept));
}

#[test]
fn filters_include_narrower_concepts() {
    let t = taxonomy();
    let (broader, narrower) = t
        .all_concepts()
        .find_map(|c| {
            let below = t.descendants(&c.scheme, &c.id);
            below.first().map(|n| (c.clone(), n.clone()))
        })
        .expect("the taxonomy has a hierarchy");
    let mut labelled = doc(1, "sk", "Škola", "Text o škole.");
    labelled.labels = vec![(broader.scheme.clone(), narrower)];
    let mut czech = doc(2, "cs", "Škola", "Text o škole.");
    czech.country = "CZ".into();
    let i = index(&[labelled, czech]);
    let filter = format!("{}/{}", broader.scheme, broader.id);
    assert_eq!(ids(&search(&i, "", &[&filter])), [1]);
    assert_eq!(ids(&search(&i, "škola", &["country/CZ"])), [2]);
    assert_eq!(
        ids(&search(&i, "škola", &["country/CZ", "country/SK"])).len(),
        2
    );
    assert!(
        search(&i, "škola", &["country/CZ", "language/sk"])
            .hits
            .is_empty()
    );
    let r = search(&i, "škola", &[]);
    assert!(
        r.facets
            .iter()
            .any(|f| f.namespace == "language" && f.value == "cs" && f.count == 1)
    );
}

#[test]
fn snippets_only_where_the_text_may_be_shown() {
    let official = doc(1, "sk", "Zákon", "Riaditeľ školy vydáva školský poriadok.");
    let copyrighted = IndexRow {
        licence: "copyrighted".into(),
        ..doc(2, "sk", "Článok", "Riaditeľ školy vydáva školský poriadok.")
    };
    let metadata_only = IndexRow {
        licence: "copyrighted".into(),
        terms: "unknown".into(),
        ..doc(
            3,
            "sk",
            "Poriadok",
            "Riaditeľ školy vydáva školský poriadok.",
        )
    };
    let i = index(&[official, copyrighted, metadata_only]);
    let r = search(&i, "riaditeľ", &[]);
    assert_eq!(ids(&r).len(), 2, "the text of document 3 is not searched");
    for hit in &r.hits {
        match hit.document_id {
            1 => {
                let s = hit.snippet.as_ref().expect("an official text is shown");
                assert!(s.text.contains("Riaditeľ"));
                assert_eq!(&s.text[s.highlighted[0].clone()], "Riaditeľ");
                assert!(hit.full_text);
            }
            _ => assert!(hit.snippet.is_none() && !hit.full_text),
        }
    }
    // the title of a metadata-only document is searched
    assert_eq!(ids(&search(&i, "poriadok", &["source/S3"])), [3]);
}

#[test]
fn noindex_and_removed_documents_are_not_found() {
    let mut hidden = doc(2, "sk", "Škola", "Škola");
    hidden.noindex = true;
    let i = index(&[doc(1, "sk", "Škola", "Škola"), hidden]);
    assert_eq!(ids(&search(&i, "škola", &[])), [1]);
    i.apply([(1, None)]).unwrap();
    assert!(search(&i, "škola", &[]).hits.is_empty());
    assert!(i.is_empty());
}

#[test]
fn authority_and_freshness_rank() {
    let mut media = doc(1, "sk", "Inklúzia", "Inklúzia v škole.");
    media.authority = "E".into();
    let ministry = doc(2, "sk", "Inklúzia", "Inklúzia v škole.");
    let mut old = doc(3, "sk", "Inklúzia", "Inklúzia v škole.");
    let mut new = doc(4, "sk", "Inklúzia", "Inklúzia v škole.");
    for (d, year) in [(&mut old, 2010), (&mut new, 2026)] {
        d.authority = "C1".into();
        d.published_at = Some(format!("{year}-09-01T00:00:00Z").parse().unwrap());
    }
    let i = index(&[media, ministry, old, new]);
    assert_eq!(ids(&search(&i, "inklúzia", &[])), [2, 4, 3, 1]);
}

#[test]
fn the_newest_documents_come_first_without_words() {
    let mut rows = Vec::new();
    for (id, year) in [(1, 2019), (2, 2026), (3, 2023)] {
        let mut d = doc(id, "sk", "Oznam", "Oznam pre školy.");
        d.published_at = Some(format!("{year}-09-01T00:00:00Z").parse().unwrap());
        rows.push(d);
    }
    let mut hidden = doc(4, "sk", "Skryté", "Nemá sa nájsť.");
    hidden.noindex = true;
    hidden.published_at = Some("2026-10-01T00:00:00Z".parse().unwrap());
    rows.push(hidden);
    let i = index(&rows);
    let newest = i.newest(&taxonomy(), &[], 10).unwrap();
    let order: Vec<i64> = newest.iter().map(|h| h.document_id).collect();
    assert_eq!(order, [2, 3, 1]);
    // the text may be shown here: the opening of it serves as the snippet
    assert!(
        newest[0]
            .snippet
            .as_ref()
            .unwrap()
            .text
            .starts_with("Oznam")
    );
    let only_sk = i.newest(&taxonomy(), &["language/en".into()], 10).unwrap();
    assert!(only_sk.is_empty());
    assert_eq!(i.newest(&taxonomy(), &[], 1).unwrap().len(), 1);
    assert!(i.newest(&taxonomy(), &["nonsense".into()], 10).is_err());
}

#[test]
fn any_word_when_not_all_match() {
    let i = index(&[doc(1, "sk", "Matematika", "Úlohy z matematiky.")]);
    let r = search(&i, "matematika", &[]);
    assert!(r.all_words);
    let r = search(&i, "matematika xyzzy", &[]);
    assert_eq!((ids(&r), r.all_words), (vec![1], false));
}

#[test]
fn chinese_words_are_found_by_pairs() {
    let i = index(&[doc(1, "zh", "阅读素养测评", "学生的阅读素养")]);
    assert_eq!(ids(&search(&i, "阅读素养", &[])), [1]);
    assert!(search(&i, "数学", &[]).hits.is_empty());
}

#[test]
fn bad_requests() {
    let i = index(&[]);
    let t = taxonomy();
    let request = |text: &str, filter: &str| SearchRequest {
        text: text.into(),
        filters: vec![filter.to_string()]
            .into_iter()
            .filter(|f| !f.is_empty())
            .collect(),
        limit: 10,
        ..SearchRequest::default()
    };
    assert!(matches!(
        i.search(&t, &request(" ", "")),
        Err(SearchError::EmptyQuery)
    ));
    assert!(matches!(
        i.search(&t, &request("x", "topic/nothing")),
        Err(SearchError::Filter(_))
    ));
    assert!(matches!(
        i.search(&t, &request("x", "nonsense")),
        Err(SearchError::Filter(_))
    ));
    let many = SearchRequest {
        text: "x".into(),
        filters: vec!["language/sk".into(); 51],
        ..SearchRequest::default()
    };
    assert!(matches!(
        i.search(&t, &many),
        Err(SearchError::TooManyFilters(50))
    ));
    // a search is cut to a reasonable length, not refused
    let long = SearchRequest {
        text: "škola ".repeat(1_000),
        ..SearchRequest::default()
    };
    assert!(i.search(&t, &long).unwrap().hits.is_empty());
}

#[test]
fn an_index_on_disk_is_only_ever_an_atena_index() {
    let root = std::env::temp_dir().join(format!("atena-search-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let path = root.join("index");
    {
        let i = SearchIndex::open(&path).unwrap();
        i.apply([(1, searchable(&doc(1, "sk", "Škola", "Škola")))])
            .unwrap();
    }
    assert_eq!(SearchIndex::open(&path).unwrap().len(), 1);
    assert!(SearchIndex::recreate(&path).unwrap().is_empty());

    let foreign = root.join("documents");
    std::fs::create_dir_all(&foreign).unwrap();
    std::fs::write(foreign.join("thesis.txt"), "mine").unwrap();
    assert!(matches!(
        SearchIndex::open(&foreign),
        Err(SearchError::NotAnIndex(_))
    ));
    assert!(matches!(
        SearchIndex::recreate(&foreign),
        Err(SearchError::NotAnIndex(_))
    ));
    assert!(foreign.join("thesis.txt").exists());
    std::fs::remove_dir_all(&root).unwrap();
}
