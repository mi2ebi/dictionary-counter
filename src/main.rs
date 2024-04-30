use std::{fs, io::Cursor, time::Instant};
use chrono::Datelike;
use xml::reader::XmlEvent;

fn main() {
    let start = Instant::now();
    let now = chrono::Utc::now();
    let current_year = now.year() as usize;
    let current_month = now.month() as usize;
    // this is a very silly way of doing things but it works
    let mut counter = vec![[(0, 0); 12]; current_year + 1];
    let client = reqwest::blocking::Client::new();
    // jvs
    let stuff = client.get("https://jbovlaste.lojban.org/export/xml-export.html?lang=en&positive_scores_only=0&bot_key=z2BsnKYJhAB0VNsl").send().unwrap().bytes().unwrap();
    let mut reader = xml::EventReader::new(Cursor::new(stuff));
    loop {
        match reader.next().unwrap() {
            XmlEvent::StartElement{name, ..} => {
                if name.local_name == "valsi" {
                    // flatness
                    for y in 2013..=current_year {
                        for m in 0..12 {
                            counter[y][m].0 += 1;
                        }
                    }
                }
            }
            XmlEvent::EndDocument => {
                break;
            }
            _ => ()
        }
    }
    // toadua
    let stuff = client.post("https://toadua.uakci.pl/api").body(r#"{"action": "search", "query": ["scope", "en"]}"#).send().unwrap();
    let stuff = serde_json::from_reader::<_, Toadua>(stuff).unwrap();
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
            jbo_t = *jbo;
            toaq_t += toaq;
            if (jbo_t > 0 || toaq_t > 0) && !(y == current_year && m > current_month) {
                out += &format!("{y}-{m:02},{jbo_t},{toaq_t}\r\n");
            }
        }
    }
    fs::write("out.csv", out).unwrap();
    println!("{:?}", Instant::now() - start);
}

#[derive(serde::Deserialize)]
struct Toadua {
    results: Vec<Toa>
}

#[derive(serde::Deserialize)]
struct Toa {
    date: String
}
