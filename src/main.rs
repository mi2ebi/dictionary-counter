use chrono::{Datelike as _, TimeZone as _, Utc};
use itertools::Itertools as _;
use regex::Regex;
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use serde::Deserialize;
use std::{
    fs,
    io::Cursor,
    time::{Duration, Instant},
};
use unicode_normalization::{char::is_combining_mark, UnicodeNormalization as _};
use xml::{reader::XmlEvent, EventReader};

#[allow(clippy::too_many_lines)]
fn main() {
    let start = Instant::now();
    let now = Utc::now();
    #[allow(clippy::cast_sign_loss)]
    let current_year = now.year() as usize;
    let current_month = now.month() as usize;
    // this is a very silly way of doing things but it works
    let mut counter = vec![[(0, 0); 12]; current_year + 1];
    let client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .unwrap();
    // jvs
    let stuff = client
        .get(format!(
            "https://jbovlaste.lojban.org/recent.html?days={:?}",
            (now - Utc::with_ymd_and_hms(&Utc {}, 2003, 1, 1, 0, 0, 0).unwrap()).num_days()
        ))
        .send()
        .unwrap()
        .text()
        .unwrap();
    let updates = Html::parse_document(&stuff);
    let sel = Selector::parse(r#"td[width="80%"]"#).unwrap();
    let mut updates = updates
        .select(&sel)
        .next()
        .unwrap()
        .text()
        .collect::<Vec<_>>();
    updates.reverse();
    // 4 dd-mmm-yyyy hh:mm:ss - definition originally entered by
    // 3 whoever
    // 2 was updated; see
    // 1 fhqwhgads
    // 0 in language newspeak
    // ^start
    let iter = updates.iter();
    let updates = iter
        .clone()
        .zip(iter.clone().skip(1).zip(iter.clone().skip(4)))
        .filter(|(_, (_, d))| d.contains("definition originally entered"))
        .map(|(l, (w, d))| (*d, (*w).to_string(), *l))
        .collect::<Vec<_>>();
    // find the ghosts
    let xml = client.get("https://jbovlaste.lojban.org/export/xml-export.html?lang=en&positive_scores_only=0&bot_key=z2BsnKYJhAB0VNsl").send().unwrap().bytes().unwrap();
    let mut reader = EventReader::new(Cursor::new(xml));
    let (mut xml_words, mut jvs_words, mut no) = (vec![], vec![], vec![]);
    let (mut in_score, mut in_def) = (false, false);
    loop {
        match reader.next().unwrap() {
            XmlEvent::EndDocument => {
                break;
            }
            XmlEvent::StartElement {
                name, attributes, ..
            } => match name.local_name.as_str() {
                "valsi" => {
                    let w = attributes
                        .iter()
                        .find(|&x| x.name.local_name == "word")
                        .unwrap()
                        .value
                        .clone();
                    if attributes
                        .iter()
                        .find(|&x| x.name.local_name == "type")
                        .unwrap()
                        .value
                        .starts_with('o')
                    {
                        no.push(w.clone());
                    } else {
                        xml_words.push(w.clone());
                    }
                }
                "score" => {
                    in_score = true;
                }
                "definition" => {
                    in_def = true;
                }
                _ => (),
            },
            XmlEvent::Characters(t) => {
                if (in_score && t.parse::<i32>().unwrap() < -1)
                    || (in_def
                        && ["with ISO 639-3", "ISO-3166", "ISO-4217"]
                            .iter()
                            .any(|i| t.contains(i)))
                {
                    no.push(xml_words.pop().unwrap_or_default());
                }
                in_score = false;
                in_def = false;
            }
            _ => (),
        }
    }
    xml_words = xml_words
        .iter()
        .map(|x| {
            Regex::new(" +")
                .unwrap()
                .replace_all(x.replace('.', " ").trim(), " ")
                .to_string()
        })
        .sorted()
        .dedup()
        .collect();
    println!("{:5} words in xml", xml_words.len());
    let mut ghosts = vec![];
    let (mut en_not_xml, mut not_en_xml, mut not_en_not_xml) = (0, 0, 0);
    let spaces = Regex::new(" +").unwrap();
    for (d, w, l) in updates {
        let w = spaces
            .replace_all(w.replace('.', " ").trim(), " ")
            .to_string();
        let date = &d.replace('\n', "")[3..11];
        let y = date[4..8].parse::<usize>().unwrap();
        let m = match &date[0..3] {
            "Jan" => 0,
            "Feb" => 1,
            "Mar" => 2,
            "Apr" => 3,
            "May" => 4,
            "Jun" => 5,
            "Jul" => 6,
            "Aug" => 7,
            "Sep" => 8,
            "Oct" => 9,
            "Nov" => 10,
            "Dec" => 11,
            x => panic!("wtf kinda month is `{x}`?"),
        };
        if l.contains("English") {
            if !jvs_words.contains(&w) && !no.contains(&w) {
                counter[y][m].0 += 1;
                jvs_words.push(w.clone());
            }
            if !xml_words.contains(&w) && !no.contains(&w) {
                ghosts.push(format!("{w} - +en-xml"));
                en_not_xml += 1;
            }
        } else if !ghosts.iter().any(|g| g.starts_with(&format!("{w} -"))) && xml_words.contains(&w)
        {
            if jvs_words.contains(&w) {
                // -g -e +x +j
                // a non english definition was added after an 'original' english one was made
            } else {
                ghosts.push(format!("{w} - -en+xml, 'first' defined in {}", &l[13..]));
                not_en_xml += 1;
                jvs_words.push(w.clone());
                counter[y][m].0 += 1;
            }
        } else if !ghosts.iter().any(|g| g.starts_with(&format!("{w} -")))
            && !xml_words.contains(&w)
            && !no.contains(&w)
        {
            ghosts.push(format!("{w} - -en-xml"));
            not_en_not_xml += 1;
        } else {
            // already a ghost
        }
    }
    println!("{:5} words in jvs (xml + ghosts)", jvs_words.len());
    let mut ghosts2 = vec![];
    for g in &ghosts {
        let w = g.split(" -").next().unwrap();
        if !ghosts2.contains(&w.to_string()) {
            ghosts2.push(g.to_string());
        }
    }
    let mut ghosts = ghosts2;
    ghosts.sort_by(|a, b| a.to_lowercase().partial_cmp(&b.to_lowercase()).unwrap());
    println!("{:5} ghost-adjacents, of which:", ghosts.len());
    println!("      {en_not_xml:5} [x] 'original' def in english   [ ] in xml   <-- real ghosts");
    println!(
        "      {not_en_xml:5} [ ]                             [x]          <-- not really ghosts"
    );
    println!(
        "      {not_en_not_xml:5} [ ]                             [ ]          <-- we don't \
         actually care about these"
    );
    let out = ghosts.join("\r\n");
    fs::write("jbosts.txt", out).unwrap();
    let out = jvs_words.iter().sorted().join("\r\n");
    fs::write("jvs.txt", out).unwrap();
    // toadua
    let mut toadua_words = vec![];
    let stuff = client
        .post("https://toadua.uakci.space/api")
        .body(r#"{"action": "search", "query": ["scope", "en"]}"#)
        .send()
        .unwrap();
    let stuff = serde_json::from_reader::<_, Toadua>(stuff).unwrap();
    for t in stuff.results {
        let the = t.date.split('-').collect::<Vec<_>>();
        let y = the[0].parse::<usize>().unwrap();
        let m = the[1].parse::<usize>().unwrap() - 1;
        if !toadua_words.contains(&t.head)
            && ![" ", ".", "@", "y", "ou", "ae", "au", "ꝡı", "ꝡu", "nhı"]
                .iter()
                .any(|x| {
                    t.head
                        .to_lowercase()
                        .nfd()
                        .filter(|&c| !is_combining_mark(c))
                        .collect::<String>()
                        .contains(x)
                })
            && !"\u{0300}\u{0303}\u{0304}\u{0309}"
                .chars()
                .any(|c| t.head.nfd().contains(&c))
            && !["oldofficial", "examples", "oldexamples"]
                .iter()
                .any(|x| t.user == *x && t.head.contains(' '))
            && t.score >= -1
        {
            toadua_words.push(t.head);
            counter[y][m].1 += 1;
        }
    }
    println!("{:5} words in toadua", toadua_words.len());
    let out = toadua_words.iter().sorted().join("\r\n");
    fs::write("toadua.txt", out).unwrap();
    // the
    let mut out = String::new();
    let (mut jbo_t, mut toaq_t) = (0, 0);
    for (y, _) in counter.iter().enumerate() {
        for (m, (jbo, toaq)) in counter[y].iter().enumerate() {
            let m = m + 1;
            jbo_t += jbo;
            toaq_t += toaq;
            if y >= 2003 && !(y == current_year && m > current_month) {
                out = format!("{y}-{m:02}\t{jbo_t}\t{toaq_t}\r\n{out}");
            }
        }
    }
    fs::write("out.tsv", out).unwrap();
    println!("{:?}", start.elapsed());
}

#[derive(Deserialize)]
struct Toadua {
    results: Vec<Toa>,
}

#[derive(Deserialize)]
struct Toa {
    date: String,
    head: String,
    user: String,
    score: i32,
}
