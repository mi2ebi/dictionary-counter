use chrono::{Datelike, TimeZone, Utc};
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use serde::Deserialize;
use std::{fs, io::Cursor, time::{Duration, Instant}};
use xml::{reader::XmlEvent, EventReader};

fn main() {
    let start = Instant::now();
    let now = Utc::now();
    let current_year = now.year() as usize;
    let current_month = now.month() as usize;
    // this is a very silly way of doing things but it works
    let mut counter = vec![[(0, 0); 12]; current_year + 1];
    let client = Client::builder().timeout(Duration::from_secs(60)).build().unwrap();
    // jvs
    let stuff = client.get(format!("https://jbovlaste.lojban.org/recent.html?days={:?}", (now - Utc::with_ymd_and_hms(&Utc{}, 2003, 1, 1, 0, 0, 0).unwrap()).num_days())).send().unwrap().text().unwrap();
    let updates = Html::parse_document(&stuff);
    let sel = Selector::parse(r#"td[width="80%"]"#).unwrap();
    let mut updates = updates.select(&sel).next().unwrap().text().collect::<Vec<_>>();
    updates.reverse();
    // 4                                                       3       2                1         0                    <--- start
    // dd-mmm-yyyy hh:mm:ss - definition originally entered by whoever was updated; see fhqwhgads in language newspeak
    let updates = updates.iter().zip(updates.iter().skip(1).zip(updates.iter().skip(4))).filter(|(_, (_, d))| d.contains("definition originally entered")).map(|(l, (w, d))| (*d, w.to_string(), *l)).collect::<Vec<_>>();
    // find the ghosts
    let xml = client.get("https://jbovlaste.lojban.org/export/xml-export.html?lang=en&positive_scores_only=0&bot_key=z2BsnKYJhAB0VNsl").send().unwrap().bytes().unwrap();
    let mut reader = EventReader::new(Cursor::new(xml));
    let mut xml_words = vec![];
    let mut jvs_words = vec![];
    loop {
        match reader.next().unwrap() {
            XmlEvent::EndDocument{..} => {
                break;
            }
            XmlEvent::StartElement{name, attributes, ..} => {
                if name.local_name == "valsi" {
                    xml_words.push(attributes.iter().find(|&x| x.name.local_name == "word").unwrap().value.clone());
                }
            }
            _ => ()
        }
    }
    println!("{:5} words in xml", xml_words.len());
    let mut ghosts = vec![];
    let (mut en_not_xml, mut not_en_xml, mut not_en_not_xml) = (0, 0, 0);
    for (d, w, l) in updates {
        if l.contains("English") {
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
                _ => panic!("wtf kinda month is `{}`?", &date[0..3])
            };
            if !jvs_words.contains(&w) {
                counter[y][m].0 += 1;
                jvs_words.push(w.clone());
            }
            if !xml_words.contains(&w) {
                ghosts.push(format!("{w} - +en-xml"));
                en_not_xml += 1;
                counter[y][m].0 += 1;
            }
        } else if !ghosts.iter().any(|g| g.starts_with(&format!("{w} -"))) && xml_words.contains(&w) {
            if !jvs_words.contains(&w) {
                ghosts.push(format!("{w} - -en+xml, 'first' defined in {}", &l[13..]));
                not_en_xml += 1;
                jvs_words.push(w);
            } else {
                // -g -e +x +j
                // a non english definition was added after an 'original' english one was made
            }
        } else if !ghosts.iter().any(|g| g.starts_with(&format!("{w} -"))) && !xml_words.contains(&w) {
            ghosts.push(format!("{w} - -en-xml"));
            not_en_not_xml += 1;
        } else {
            // already a ghost
        }
    }
    println!("{:5} english words in jvs changelog", jvs_words.len());
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
    println!("      {not_en_xml:5} [ ]                             [x]          <-- not really ghosts");
    println!("      {not_en_not_xml:5} [ ]                             [ ]          <-- we don't actually care about these");
    let out = ghosts.join("\r\n");
    fs::write("jbosts.txt", out).unwrap();
    // toadua
    let stuff = client.post("https://toadua.uakci.pl/api").body(r#"{"action": "search", "query": ["scope", "en"]}"#).send().unwrap();
    let stuff = serde_json::from_reader::<_, Toadua>(stuff).unwrap();
    println!("{:5} words in toadua", stuff.results.len());
    for t in stuff.results {
        let the = t.date.split('-').collect::<Vec<_>>();
        let y = the[0].parse::<usize>().unwrap();
        let m = the[1].parse::<usize>().unwrap() - 1;
        counter[y][m].1 += 1;
    }
    let mut out = String::new();
    let (mut jbo_t, mut toaq_t) = (0, 0);
    for (y, _) in counter.iter().enumerate() {
        for (m, (jbo, toaq)) in counter[y].iter().enumerate() {
            let m = m + 1;
            jbo_t += jbo;
            toaq_t += toaq;
            if y >= 2003 && !(y == current_year && m > current_month) {
                out += &format!("{y}-{m:02}\t{jbo_t}\t{toaq_t}\r\n");
            }
        }
    }
    fs::write("out.tsv", out).unwrap();
    println!("{:?}", Instant::now() - start);
}

#[derive(Deserialize)]
struct Toadua {
    results: Vec<Toa>
}

#[derive(Deserialize)]
struct Toa {
    date: String
}
