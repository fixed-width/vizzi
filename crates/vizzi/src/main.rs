//! Local HTTP host and socket-free diagnostic entry point.

use std::{
    io::{BufRead, Write},
    path::PathBuf,
    sync::Arc,
};
use vizzi::admission::Admission;
use vizzi::syntax_cache::SyntaxCache;
use vizzi::{cache::Cache, engine::Engine, host::Host, source::Directory};
use vizzi_core::analysis::Languages;

mod configuration;

#[tokio::main(worker_threads = 2)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(options) = configuration::load(std::env::args().skip(1))? else {
        print!("{}", include_str!("cli-help.txt"));
        return Ok(());
    };
    let policy = options.policy()?;
    let configuration::Options {
        root,
        cache,
        workers,
        port,
        stdio,
        once,
        syntax_cache_mib,
        analysis_input_mib,
        source_spool_mib,
        investigation_file,
        editor_code,
        language_servers,
        ..
    } = options;
    let mut directory = Directory::with_policy(
        &root.unwrap_or_else(|| PathBuf::from(".")),
        policy,
        Arc::new(Languages::builtin()?),
    )?;
    let cache = match cache {
        Some(path) => vizzi::cache::external_path(&path, directory.root())?,
        None => vizzi::cache::default_path(directory.root())?,
    };
    let investigation_file = investigation_file
        .map(|path| vizzi::cache::external_path(&path, directory.root()))
        .transpose()?;
    if investigation_file
        .as_ref()
        .is_some_and(|path| path.starts_with(&cache))
    {
        return Err("Investigation file must be outside the analysis cache".into());
    }
    if source_spool_mib != 0 {
        let capacity = source_spool_mib
            .checked_mul(1024 * 1024)
            .ok_or("source spool capacity overflow")?;
        directory = directory.with_spool(Arc::new(vizzi::source_spool::SourceSpool::new(
            &cache, capacity,
        )?))?;
    }
    let definitions = if language_servers.is_empty() {
        if language_servers.python_path.is_some() || language_servers.compilation_database.is_some()
        {
            return Err("Language-server project options require a configured server".into());
        }
        None
    } else {
        Some(Arc::new(vizzi::language_servers::Servers::new(
            directory.root().to_owned(),
            cache.join("semantic"),
            language_servers,
        )?) as Arc<dyn vizzi::definitions::Provider>)
    };
    let mut engine = Engine::new(directory, Cache::new(cache), workers)?;
    if analysis_input_mib != 0 {
        let bytes = analysis_input_mib
            .checked_mul(1024 * 1024)
            .ok_or("analysis input budget overflow")?;
        engine = engine.with_admission(Arc::new(Admission::for_budget(bytes)?));
    }
    if syntax_cache_mib != 0 {
        let budget = syntax_cache_mib
            .checked_mul(1024 * 1024)
            .ok_or("syntax cache budget overflow")?;
        engine = engine.with_syntax_cache(Arc::new(SyntaxCache::for_budget(budget)?));
    }
    if once {
        println!(
            "{}",
            serde_json::to_string(&vizzi::host::one_shot(&engine)?.status())?
        );
        return Ok(());
    }
    let host = Arc::new(if stdio {
        Host::start(engine)?
    } else {
        Host::prepare(engine)?
    });
    let application = host.application();
    let reviews = Arc::new(vizzi::review_application::ReviewApplication::start(
        host.clone(),
        vizzi::review_source::CaptureLimits::default(),
    )?);
    let investigation = Arc::new(vizzi::companion::Companion::new(
        host.clone(),
        investigation_file,
        Arc::new(vizzi::editor::VsCode::new(editor_code)),
    )?);
    let router = vizzi::server::router_with_definitions(
        host.clone(),
        reviews.clone(),
        investigation,
        definitions,
    );
    if stdio {
        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout().lock();
        for line in stdin.lock().lines() {
            let line = line?;
            #[derive(serde::Deserialize)]
            struct Request {
                id: serde_json::Value,
                uri: String,
                method: Option<String>,
                body: Option<serde_json::Value>,
            }
            let response = match serde_json::from_str::<Request>(&line) {
                Ok(request) => {
                    match vizzi::server::diagnostic_request(
                        router.clone(),
                        request.id,
                        request.method.as_deref().unwrap_or("GET"),
                        &request.uri,
                        request.body,
                    )
                    .await
                    {
                        Ok(response) => response,
                        Err(error) => vizzi::server::DiagnosticResponse {
                            id: serde_json::Value::Null,
                            status: 400,
                            content_type: "text/plain".into(),
                            body: error.to_string(),
                        },
                    }
                }
                Err(error) => vizzi::server::DiagnosticResponse {
                    id: serde_json::Value::Null,
                    status: 400,
                    content_type: "text/plain".into(),
                    body: error.to_string(),
                },
            };
            writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
            stdout.flush()?;
        }
    } else {
        let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
        eprintln!("Vizzi: http://{}", listener.local_addr()?);
        axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                let _ = tokio::signal::ctrl_c().await;
                application.close_updates();
                reviews.shutdown();
            })
            .await?;
    }
    Ok(())
}
