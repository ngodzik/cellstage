//! What the window looks up on the web for the agent, each from a source
//! whose terms allow it, and kept in the user's cache, never in the
//! project: pictures from Wikimedia Commons, proteins from UniProt, the
//! Protein Data Bank and the AlphaFold database, pathways from Reactome.

use std::path::PathBuf;

use serde_json::Value;

/// Who is asking, as Wikimedia asks every program to say.
const AGENT: &str = "cellstage/0.1 (a local window for learning biology)";

fn cache(kind: &str) -> PathBuf {
    crate::protein::cache()
        .parent()
        .map_or_else(std::env::temp_dir, PathBuf::from)
        .join(kind)
}

/// Text made safe for a URL's query.
fn encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// The bytes at a URL; asked a second time, after a pause, when the server
/// says it is busy.
fn get_bytes(url: &str) -> Result<Vec<u8>, String> {
    let mut tries = 0;
    loop {
        tries += 1;
        let answer = ureq::get(url).header("User-Agent", AGENT).call();
        match answer {
            Ok(mut response) => {
                return response
                    .body_mut()
                    .with_config()
                    .limit(60_000_000)
                    .read_to_vec()
                    .map_err(|e| format!("reading from {}: {e}", host(url)));
            }
            Err(ureq::Error::StatusCode(429 | 503)) if tries < 3 => {
                std::thread::sleep(std::time::Duration::from_millis(1500 * tries));
            }
            Err(e) => return Err(format!("asking {}: {e}", host(url))),
        }
    }
}

fn get_text(url: &str) -> Result<String, String> {
    String::from_utf8(get_bytes(url)?).map_err(|_| {
        format!(
            "{} answered with something that is not text; it may be down",
            host(url)
        )
    })
}

fn get_json(url: &str) -> Result<Value, String> {
    let text = get_text(url)?;
    serde_json::from_str(&text).map_err(|_| format!("{} did not answer in JSON", host(url)))
}

fn post_json(url: &str, body: &Value) -> Result<Value, String> {
    let text = ureq::post(url)
        .header("User-Agent", AGENT)
        .header("Content-Type", "application/json")
        .send(body.to_string())
        .map_err(|e| format!("asking {}: {e}", host(url)))?
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("reading from {}: {e}", host(url)))?;
    serde_json::from_str(&text).map_err(|_| format!("{} did not answer in JSON", host(url)))
}

fn host(url: &str) -> &str {
    url.split('/').nth(2).unwrap_or(url)
}

/// Text without its HTML tags, entities read, spaces gathered.
pub fn plain(html: &str) -> String {
    let mut out = String::new();
    let mut inside = false;
    for c in html.chars() {
        match c {
            '<' => inside = true,
            '>' => inside = false,
            c if !inside => out.push(c),
            _ => {}
        }
    }
    let out = out
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ");
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn short(text: &str, most: usize) -> String {
    if text.chars().count() <= most {
        return text.to_owned();
    }
    let cut: String = text.chars().take(most).collect();
    format!("{}…", cut.trim_end())
}

/// A picture from Wikimedia Commons, with who made it and under which
/// licence; `path` is where it is kept, once fetched.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Picture {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub author: String,
    pub licence: String,
    pub description: String,
    pub page: String,
    pub url: String,
    #[serde(default)]
    pub path: PathBuf,
}

fn picture(page: &Value) -> Option<Picture> {
    let info = page.get("imageinfo")?.get(0)?;
    let meta = info.get("extmetadata")?;
    let field = |k: &str| {
        meta.get(k)
            .and_then(|m| m.get("value"))
            .and_then(Value::as_str)
            .map(plain)
            .unwrap_or_default()
    };
    let mime = info.get("thumbmime").or(info.get("mime"))?.as_str()?;
    let url = info.get("thumburl").or(info.get("url"))?.as_str()?;
    // Only what the renderer reads.
    let readable = ["image/png", "image/jpeg", "image/gif", "image/webp"].contains(&mime)
        || url.split('?').next().is_some_and(|u| {
            [".png", ".jpg", ".jpeg", ".gif", ".webp"]
                .iter()
                .any(|e| u.to_lowercase().ends_with(e))
        });
    if !readable {
        return None;
    }
    Some(Picture {
        title: page.get("title")?.as_str()?.to_owned(),
        width: info.get("thumbwidth").or(info.get("width"))?.as_u64()? as u32,
        height: info.get("thumbheight").or(info.get("height"))?.as_u64()? as u32,
        author: field("Artist"),
        licence: field("LicenseShortName"),
        description: short(&field("ImageDescription"), 300),
        page: info
            .get("descriptionurl")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned(),
        url: url.to_owned(),
        path: PathBuf::new(),
    })
}

const COMMONS: &str = "https://commons.wikimedia.org/w/api.php?action=query&format=json&prop=imageinfo&iiprop=url|size|mime|extmetadata&iiurlwidth=1600&iiextmetadatafilter=LicenseShortName|Artist|ImageDescription";

/// Pictures on Wikimedia Commons that match a search: photographs,
/// micrographs, old anatomical plates, drawings, each with its licence.
pub fn commons_search(query: &str, limit: usize) -> Result<Vec<Picture>, String> {
    let url = format!(
        "{COMMONS}&generator=search&gsrnamespace=6&gsrlimit={}&gsrsearch={}",
        limit.clamp(1, 20),
        encode(&format!("filetype:bitmap|drawing {query}"))
    );
    let answer = get_json(&url)?;
    let mut found: Vec<(i64, Picture)> = answer
        .get("query")
        .and_then(|q| q.get("pages"))
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter_map(|(_, page)| {
            let index = page.get("index").and_then(Value::as_i64).unwrap_or(0);
            picture(page).map(|p| (index, p))
        })
        .collect();
    found.sort_by_key(|(i, _)| *i);
    Ok(found.into_iter().map(|(_, p)| p).collect())
}

/// A picture from Commons by its file name, fetched once and kept.
pub fn commons_picture(title: &str) -> Result<Picture, String> {
    let title = title.trim();
    let title = if title.starts_with("File:") {
        title.to_owned()
    } else {
        format!("File:{title}")
    };
    let safe: String = title[5..]
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '.' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .take(120)
        .collect();
    let dir = cache("commons");
    let sidecar = dir.join(format!("{safe}.json"));
    if let Some(p) = std::fs::read_to_string(&sidecar)
        .ok()
        .and_then(|t| serde_json::from_str::<Picture>(&t).ok())
        .filter(|p| p.path.exists())
    {
        return Ok(p);
    }
    let answer = get_json(&format!("{COMMONS}&titles={}", encode(&title)))?;
    let page = answer
        .get("query")
        .and_then(|q| q.get("pages"))
        .and_then(Value::as_object)
        .and_then(|m| m.values().next())
        .ok_or("Commons did not answer")?;
    if page.get("missing").is_some() {
        return Err(format!(
            "Commons has no {title}; image_search finds file names"
        ));
    }
    let mut p =
        picture(page).ok_or_else(|| format!("{title} is not a picture this window reads"))?;
    let bytes = get_bytes(&p.url).map_err(|e| format!("fetching {title}: {e}"))?;
    let ext = p
        .url
        .split('?')
        .next()
        .and_then(|u| u.rsplit('.').next())
        .unwrap_or("png")
        .to_lowercase();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    p.path = dir.join(format!("{safe}.{ext}"));
    std::fs::write(&p.path, &bytes).map_err(|e| e.to_string())?;
    if let Ok(text) = serde_json::to_string(&p) {
        let _ = std::fs::write(&sidecar, text);
    }
    Ok(p)
}

/// A drawing whose `<image href="commons:File:...">` are made to point
/// at the pictures, fetched.
pub fn resolve_commons(svg: &str) -> Result<String, String> {
    let mut out = String::with_capacity(svg.len());
    let mut rest = svg;
    while let Some(at) = rest.find("\"commons:") {
        out.push_str(&rest[..at + 1]);
        let after = &rest[at + 9..];
        let end = after.find('"').ok_or("an href that does not close")?;
        let picture = commons_picture(&after[..end])?;
        out.push_str(&crate::drawing::text::escape(
            &picture.path.to_string_lossy(),
        ));
        rest = &after[end..];
    }
    out.push_str(rest);
    Ok(out)
}

/// Proteins that match a name, a gene, or a subject: human entries of
/// UniProt with what each does, which the AlphaFold database has a
/// predicted structure for, and measured structures in the PDB.
pub fn proteins(query: &str) -> Result<String, String> {
    let mut out = String::new();
    let human = get_json(&format!(
        "https://rest.uniprot.org/uniprotkb/search?format=json&size=5&fields=accession,protein_name,gene_primary,length,cc_function&query={}",
        encode(&format!("({query}) AND organism_id:9606 AND reviewed:true"))
    ));
    match human {
        Ok(answer) => {
            let results = answer
                .get("results")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            if results.is_empty() {
                out.push_str("UniProt: no reviewed human protein matches.\n");
            } else {
                out.push_str("Human proteins in UniProt (show_protein uniprot=... shows the AlphaFold prediction):\n");
            }
            for r in results {
                let accession = r
                    .get("primaryAccession")
                    .and_then(Value::as_str)
                    .unwrap_or("?");
                let name = r
                    .pointer("/proteinDescription/recommendedName/fullName/value")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let gene = r
                    .pointer("/genes/0/geneName/value")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let length = r
                    .pointer("/sequence/length")
                    .and_then(Value::as_u64)
                    .unwrap_or(0);
                let function = r
                    .get("comments")
                    .and_then(Value::as_array)
                    .and_then(|c| {
                        c.iter().find(|x| {
                            x.get("commentType").and_then(Value::as_str) == Some("FUNCTION")
                        })
                    })
                    .and_then(|c| c.pointer("/texts/0/value"))
                    .and_then(Value::as_str)
                    .map(|t| short(&strip_evidence(t), 420))
                    .unwrap_or_default();
                out.push_str(&format!(
                    "- {accession} {name} (gene {gene}, {length} residues). {function}\n"
                ));
            }
        }
        Err(e) => out.push_str(&format!("UniProt did not answer ({e}).\n")),
    }
    let search = post_json(
        "https://search.rcsb.org/rcsbsearch/v2/query",
        &serde_json::json!({
            "query": {"type": "terminal", "service": "full_text", "parameters": {"value": query}},
            "return_type": "entry",
            "request_options": {"paginate": {"start": 0, "rows": 8}}
        }),
    );
    let ids: Vec<String> = search
        .as_ref()
        .ok()
        .and_then(|a| a.get("result_set"))
        .and_then(Value::as_array)
        .map(|r| {
            r.iter()
                .filter_map(|x| x.get("identifier")?.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    if ids.is_empty() {
        out.push_str("PDB: no entry found.\n");
        return Ok(out);
    }
    let list = ids
        .iter()
        .map(|i| format!("\"{i}\""))
        .collect::<Vec<_>>()
        .join(",");
    let details = post_json(
        "https://data.rcsb.org/graphql",
        &serde_json::json!({"query": format!(
            "{{entries(entry_ids:[{list}]){{rcsb_id struct{{title}} exptl{{method}} rcsb_entry_info{{resolution_combined deposited_atom_count}} polymer_entities{{rcsb_entity_source_organism{{scientific_name}}}}}}}}"
        )}),
    )?;
    out.push_str("Measured structures in the PDB (show_protein pdb_id=...):\n");
    for e in details
        .pointer("/data/entries")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let id = e.get("rcsb_id").and_then(Value::as_str).unwrap_or("?");
        let title = e
            .pointer("/struct/title")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_lowercase();
        let method = e
            .pointer("/exptl/0/method")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_lowercase();
        let resolution = e
            .pointer("/rcsb_entry_info/resolution_combined/0")
            .and_then(Value::as_f64)
            .map(|r| format!(", {r:.1} Å"))
            .unwrap_or_default();
        let atoms = e
            .pointer("/rcsb_entry_info/deposited_atom_count")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let mut organisms: Vec<&str> = e
            .get("polymer_entities")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|p| {
                p.pointer("/rcsb_entity_source_organism/0/scientific_name")?
                    .as_str()
            })
            .collect();
        organisms.dedup();
        let too_big = if atoms > 99_999 {
            "; too large for this window (mmCIF only)"
        } else {
            ""
        };
        out.push_str(&format!(
            "- {id}: {title} ({method}{resolution}; {}{too_big})\n",
            organisms.join(", ")
        ));
    }
    Ok(out)
}

/// UniProt's text without its references to evidence, "(PubMed:123)".
fn strip_evidence(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(i) = rest.find(" (PubMed:") {
        out.push_str(&rest[..i]);
        rest = rest[i..].find(')').map_or("", |j| &rest[i + j + 1..]);
    }
    out.push_str(rest);
    out
}

/// What the AlphaFold database predicts for a UniProt entry.
pub struct Prediction {
    pub text: String,
    pub name: String,
    /// The mean confidence, pLDDT, from 0 to 100.
    pub confidence: f32,
    /// Stretches of residues predicted with low confidence, below 50.
    pub loose: Vec<(i32, i32)>,
}

/// The AlphaFold database's predicted structure of a protein, fetched
/// once and kept. The database is under CC BY 4.0.
pub fn predicted(accession: &str) -> Result<Prediction, String> {
    let accession = accession.trim().to_ascii_uppercase();
    if !(6..=10).contains(&accession.len()) || !accession.chars().all(|c| c.is_ascii_alphanumeric())
    {
        return Err(format!(
            "a UniProt accession is like P04637; got {accession}"
        ));
    }
    let dir = crate::protein::cache();
    let file = dir.join(format!("AF-{accession}.pdb"));
    let name_file = dir.join(format!("AF-{accession}.name"));
    let (text, name) = match (
        std::fs::read_to_string(&file),
        std::fs::read_to_string(&name_file),
    ) {
        (Ok(t), Ok(n)) => (t, n),
        _ => {
            let answer = get_json(&format!(
                "https://alphafold.ebi.ac.uk/api/prediction/{accession}"
            ))
            .map_err(|e| {
                format!("{e}; the AlphaFold database may have no prediction for {accession}")
            })?;
            let entry = answer
                .get(0)
                .ok_or_else(|| format!("no AlphaFold prediction for {accession}"))?;
            let url = entry
                .get("pdbUrl")
                .and_then(Value::as_str)
                .ok_or("no PDB file in the prediction")?;
            let name = format!(
                "{} ({})",
                entry
                    .get("uniprotDescription")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                entry.get("gene").and_then(Value::as_str).unwrap_or("")
            );
            let text = get_text(url)?;
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            std::fs::write(&file, &text).map_err(|e| e.to_string())?;
            let _ = std::fs::write(&name_file, &name);
            (text, name)
        }
    };
    // The confidence of each residue is written where a measured structure
    // has its B-factor.
    let mut residues: Vec<(i32, f32)> = Vec::new();
    for line in text
        .lines()
        .filter(|l| l.starts_with("ATOM  ") && l.get(12..16) == Some(" CA "))
    {
        if let (Some(n), Some(b)) = (
            line.get(22..26).and_then(|s| s.trim().parse().ok()),
            line.get(60..66).and_then(|s| s.trim().parse().ok()),
        ) {
            residues.push((n, b));
        }
    }
    let confidence = residues.iter().map(|r| r.1).sum::<f32>() / residues.len().max(1) as f32;
    let mut loose: Vec<(i32, i32)> = Vec::new();
    for (n, b) in &residues {
        if *b < 50.0 {
            match loose.last_mut() {
                Some(last) if last.1 + 1 == *n => last.1 = *n,
                _ => loose.push((*n, *n)),
            }
        }
    }
    loose.retain(|(a, b)| b - a >= 4);
    Ok(Prediction {
        text,
        name,
        confidence,
        loose,
    })
}

/// Pathways in Reactome that match a subject, or one pathway or reaction
/// by its id: what it is, and the steps it holds. Reactome is under CC BY
/// 4.0.
pub fn reactome(query: &str) -> Result<String, String> {
    let q = query.trim();
    if q.starts_with("R-HSA-") {
        let e = get_json(&format!(
            "https://reactome.org/ContentService/data/query/{}",
            encode(q)
        ))?;
        let name = e.get("displayName").and_then(Value::as_str).unwrap_or("");
        let class = e.get("schemaClass").and_then(Value::as_str).unwrap_or("");
        let mut out = format!("{q} {name} ({class}).\n");
        if let Some(t) = e.pointer("/summation/0/text").and_then(Value::as_str) {
            out.push_str(&format!("{}\n", short(&plain(t), 900)));
        }
        let names = |key: &str| -> Vec<String> {
            e.get(key)
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|x| {
                    x.get("displayName")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                })
                .collect()
        };
        for (key, label) in [("input", "Inputs"), ("output", "Outputs")] {
            let n = names(key);
            if !n.is_empty() {
                out.push_str(&format!("{label}: {}\n", n.join("; ")));
            }
        }
        let catalysts: Vec<String> = e
            .get("catalystActivity")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|c| {
                c.pointer("/physicalEntity/displayName")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .collect();
        if !catalysts.is_empty() {
            out.push_str(&format!("Catalysed by: {}\n", catalysts.join("; ")));
        }
        let steps: Vec<String> = e
            .get("hasEvent")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|x| {
                Some(format!(
                    "- {} {} ({})",
                    x.get("stId")?.as_str()?,
                    x.get("displayName")?.as_str()?,
                    x.get("schemaClass")?.as_str()?
                ))
            })
            .collect();
        if !steps.is_empty() {
            out.push_str(&format!("Steps it holds:\n{}\n", steps.join("\n")));
        }
        return Ok(out);
    }
    let answer = get_json(&format!(
        "https://reactome.org/ContentService/search/query?query={}&species=Homo%20sapiens&types=Pathway&types=Reaction&cluster=true",
        encode(q)
    ))
    .map_err(|e| format!("{e}; Reactome found nothing, or did not answer"))?;
    let mut out = String::from("Reactome (give an id to pathway_lookup for its steps):\n");
    // Pathways first: a word matches many reactions loosely.
    let groups: Vec<&Value> = answer
        .get("results")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .collect();
    for (kind, most) in [("Pathway", 8), ("Reaction", 4)] {
        let entries = groups
            .iter()
            .filter(|g| g.get("typeName").and_then(Value::as_str) == Some(kind))
            .flat_map(|g| {
                g.get("entries")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
            })
            .filter(|e| {
                e.get("stId")
                    .and_then(Value::as_str)
                    .is_some_and(|id| id.starts_with("R-HSA-"))
            })
            .take(most);
        for e in entries {
            let id = e.get("stId").and_then(Value::as_str).unwrap_or("?");
            let name = plain(e.get("name").and_then(Value::as_str).unwrap_or(""));
            let summary = short(
                &plain(e.get("summation").and_then(Value::as_str).unwrap_or("")),
                200,
            );
            out.push_str(&format!("- {id} {name} ({kind}). {summary}\n"));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_becomes_plain() {
        assert_eq!(
            plain("<span class=\"x\">Apoptosis</span> &amp; more\n  text"),
            "Apoptosis & more text"
        );
        assert_eq!(encode("a b/é"), "a%20b%2F%C3%A9");
        assert_eq!(
            strip_evidence("Binds DNA (PubMed:123, PubMed:456). Acts."),
            "Binds DNA. Acts."
        );
    }
}
