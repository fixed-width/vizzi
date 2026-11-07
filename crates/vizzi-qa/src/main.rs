//! Socket-free experimental retrieval/provenance process; inference remains external.
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    io::{BufRead, Read, Write},
    path::PathBuf,
    time::Instant,
};
use vizzi_qa::{Answer, Context, Corpus, IdAnswer, SourceRange, native::capture};
#[derive(Default, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Selector {
    #[default]
    Legacy,
    Compact,
    Search,
}
fn default_limit() -> usize {
    6
}
fn default_budget() -> usize {
    8000
}
#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
enum Request {
    Retrieve {
        id: String,
        question: String,
        #[serde(default)]
        selector: Selector,
        #[serde(default = "default_limit")]
        limit: usize,
        #[serde(default = "default_budget")]
        budget: usize,
    },
    Oracle {
        id: String,
        anchors: Vec<(String, String)>,
    },
    Select {
        id: String,
        snapshot: String,
        ranges: Vec<SourceRange>,
        budget: usize,
    },
    Validate {
        id: String,
        answer: Answer,
    },
    ValidateIds {
        id: String,
        answer: IdAnswer,
    },
    Reorder {
        id: String,
        order: Vec<usize>,
    },
    Refresh,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(args.next().ok_or("usage: vizzi-qa ROOT")?);
    if args.next().is_some() {
        return Err("usage: vizzi-qa ROOT".into());
    }
    let mut corpus = Corpus::new()?;
    let begin = Instant::now();
    let (files, capture_stats) = capture(&root)?;
    let stats = corpus.replace(files)?;
    let mut stdout = std::io::stdout().lock();
    writeln!(
        stdout,
        "{}",
        json!({"schema":1,"ready":true,"snapshot":corpus.snapshot(),"stats":stats,"capture":capture_stats,"index_ms":begin.elapsed().as_secs_f64()*1000.0})
    )?;
    stdout.flush()?;
    let mut input = std::io::stdin().lock();
    let mut contexts: VecDeque<(String, Context)> = VecDeque::new();
    loop {
        let mut bytes = vec![];
        let n = Read::take(&mut input, 128 * 1024 + 1).read_until(b'\n', &mut bytes)?;
        if n == 0 {
            break;
        }
        if n > 128 * 1024 {
            return Err("QA request exceeds 128 KiB".into());
        }
        let begin = Instant::now();
        let result = (|| -> Result<Value, Box<dyn std::error::Error>> {
            let request: Request = serde_json::from_slice(&bytes)?;
            match request {
                Request::Retrieve {
                    id,
                    question,
                    selector,
                    limit,
                    budget,
                } => {
                    if id.len() > 256 {
                        return Err("request id exceeds 256 bytes".into());
                    }
                    let context = match selector {
                        Selector::Legacy => corpus.retrieve(&question, limit, budget)?,
                        Selector::Compact => corpus.retrieve_compact(&question, limit, budget)?,
                        Selector::Search => corpus.retrieve_search(&question, limit, budget)?,
                    };
                    let response = json!({"id":id,"context":context});
                    contexts.push_back((id, context));
                    Ok(response)
                }
                Request::Oracle { id, anchors } => {
                    let context = corpus.oracle(&anchors, 8000)?;
                    let response = json!({"id":id,"context":context});
                    contexts.push_back((id, context));
                    Ok(response)
                }
                Request::Select {
                    id,
                    snapshot,
                    ranges,
                    budget,
                } => {
                    if id.len() > 256 {
                        return Err("request id exceeds 256 bytes".into());
                    }
                    let context = corpus.select_ranges(&snapshot, &ranges, budget)?;
                    let response = json!({"id":id,"context":context});
                    contexts.push_back((id, context));
                    Ok(response)
                }
                Request::Validate { id, answer } => {
                    let context = &contexts
                        .iter()
                        .rev()
                        .find(|(key, _)| key == &id)
                        .ok_or("Context expired or unknown")?
                        .1;
                    corpus.validate(context, &answer)?;
                    Ok(
                        json!({"id":id,"provenance_valid":true,"semantic_correctness":"not_checked"}),
                    )
                }
                Request::ValidateIds { id, answer } => {
                    let context = &contexts
                        .iter()
                        .rev()
                        .find(|(key, _)| key == &id)
                        .ok_or("Context expired or unknown")?
                        .1;
                    let references = corpus.validate_ids(context, &answer)?;
                    Ok(
                        json!({"id":id,"provenance_valid":true,"references":references,"semantic_correctness":"not_checked"}),
                    )
                }
                Request::Reorder { id, order } => {
                    let context = &contexts
                        .iter()
                        .rev()
                        .find(|(key, _)| key == &id)
                        .ok_or("Context expired or unknown")?
                        .1;
                    let evidence = corpus.reorder(context, &order)?;
                    Ok(
                        json!({"id":id,"context":{"snapshot":context.snapshot(),"evidence":evidence},"relevance":"not_checked"}),
                    )
                }
                Request::Refresh => {
                    let (files, capture_stats) = capture(&root)?;
                    let stats = corpus.replace(files)?;
                    Ok(json!({"snapshot":corpus.snapshot(),"stats":stats,"capture":capture_stats}))
                }
            }
        })();
        while contexts.len() > 64 {
            contexts.pop_front();
        }
        let mut response = match result {
            Ok(value) => value,
            Err(e) => json!({"error":e.to_string()}),
        };
        response["elapsed_ms"] = json!(begin.elapsed().as_secs_f64() * 1000.0);
        writeln!(stdout, "{response}")?;
        stdout.flush()?;
    }
    Ok(())
}
