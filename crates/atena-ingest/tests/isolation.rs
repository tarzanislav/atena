//! Parsing in a limited child process (pillar 3): what a hostile document does to the parser stays
//! in the child – the crawler gets an answer, never a crash or a hang.

use std::time::{Duration, Instant};

use atena_extract::{ExtractError, Parser, extract, legal_signals};
use url::Url;

fn worker(memory_limit: usize, seconds: u64) -> Parser {
    Parser::Isolated {
        program: env!("CARGO_BIN_EXE_atena-ingest").into(),
        memory_limit,
        time_limit: Duration::from_secs(seconds),
    }
}

fn url() -> Url {
    Url::parse("https://www.example.sk/oznamy/").unwrap()
}

const PAGE: &str = "<html lang=\"sk\"><head><title>Oznam</title></head><body><main>\
    <h1>Zápis do prvého ročníka</h1><p>Zápis detí do prvého ročníka základnej školy sa koná v apríli.</p>\
    <footer>© 2026 ZŠ Príkladná. <a href=\"/podmienky\">Podmienky používania</a></footer></main></body></html>";

/// A small PDF whose page content inflates to 200 MB.
fn pdf_bomb() -> Vec<u8> {
    use std::io::Write;

    let mut zip = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
    let spaces = vec![b' '; 1 << 20];
    for _ in 0..200 {
        zip.write_all(&spaces).unwrap();
    }
    let content = zip.finish().unwrap();
    let objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R >>".to_vec(),
        [
            format!(
                "<< /Length {} /Filter /FlateDecode >>\nstream\n",
                content.len()
            )
            .into_bytes(),
            content,
            b"\nendstream".to_vec(),
        ]
        .concat(),
    ];
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend(format!("{} 0 obj\n", i + 1).as_bytes());
        pdf.extend(body);
        pdf.extend(b"\nendobj\n");
    }
    let xref = pdf.len();
    pdf.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
    for offset in offsets {
        pdf.extend(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    pdf
}

#[tokio::test]
async fn an_ordinary_page_is_read_in_the_child_as_in_the_process() {
    let parser = worker(atena_extract::MEMORY_LIMIT, 60);
    let isolated = parser
        .extract(PAGE.as_bytes().to_vec(), Some("text/html"), &url())
        .await
        .unwrap();
    assert_eq!(
        isolated,
        extract(PAGE.as_bytes(), Some("text/html"), &url()).unwrap()
    );
    let signals = parser
        .legal_signals(PAGE.as_bytes().to_vec(), Some("text/html"), &url())
        .await
        .unwrap();
    assert_eq!(
        signals,
        legal_signals(PAGE.as_bytes(), Some("text/html"), &url())
    );
    // refusals travel back as they are
    assert!(matches!(
        parser
            .extract(b"MZ\x90\x00".to_vec(), Some("application/pdf"), &url())
            .await,
        Err(ExtractError::Dangerous(_))
    ));
}

#[tokio::test]
async fn a_memory_bomb_stops_the_child_not_the_crawler() {
    let bomb = pdf_bomb();
    assert!(bomb.len() < 1024 * 1024, "{}", bomb.len());
    let started = Instant::now();
    let result = worker(64 * 1024 * 1024, 60)
        .extract(bomb, Some("application/pdf"), &url())
        .await;
    assert!(
        matches!(result, Err(ExtractError::TooDemanding(_))),
        "{result:?}"
    );
    assert!(started.elapsed() < Duration::from_secs(50));
}

#[tokio::test]
async fn a_parser_that_takes_too_long_is_killed() {
    let bomb = pdf_bomb();
    let started = Instant::now();
    // memory is no limit here: the clock stops it
    let result = worker(4 * 1024 * 1024 * 1024, 1)
        .extract(bomb, Some("application/pdf"), &url())
        .await;
    assert!(
        matches!(result, Err(ExtractError::TooDemanding(_))),
        "{result:?}"
    );
    assert!(started.elapsed() < Duration::from_secs(10));
}
