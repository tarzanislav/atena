//! Pages that are mostly about people – galleries, staff lists, contacts, accounts. ATENA does not
//! follow links to them and does not accept them as sources (data minimisation, GDPR Art. 5(1)(c)).

/// Path words of pages that are mostly about people (galleries, staff lists, contacts, accounts);
/// ATENA does not follow links to them (data minimisation).
const PERSONAL_WORDS: [&str; 30] = [
    "foto",
    "fotky",
    "fotografie",
    "fotoalbum",
    "photo",
    "photos",
    "gallery",
    "album",
    "albums",
    "staff",
    "team",
    "people",
    "osoby",
    "ziaci",
    "zaci",
    "absolventi",
    "alumni",
    "profil",
    "profile",
    "user",
    "users",
    "uzivatel",
    "clenovia",
    "members",
    "login",
    "prihlasenie",
    "registracia",
    "kolektiv",
    "ucitelia",
    "pedagogovia",
];
/// Beginnings of path words with the same meaning (`galeria`, `zamestnanci`, `kontakty`…).
const PERSONAL_STEMS: [&str; 6] = [
    "galeri",
    "fotogaleri",
    "zamestnan",
    "kontakt",
    "contact",
    "employee",
];

/// `true` for URL paths of pages that are mostly about people.
pub fn is_personal_path(path: &str) -> bool {
    path.split(|c: char| !c.is_ascii_alphanumeric())
        .map(str::to_ascii_lowercase)
        .any(|w| {
            PERSONAL_WORDS.contains(&w.as_str()) || PERSONAL_STEMS.iter().any(|s| w.starts_with(s))
        })
}
