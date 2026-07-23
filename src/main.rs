use acli_rust::agile;
use acli_rust::alerts;
use acli_rust::bitbucket;
use acli_rust::client::Client;
use acli_rust::config::{self, Config};
use acli_rust::confluence;
use acli_rust::jira;
use std::io::Read;

fn main() {
    let mut args: Vec<String> = std::env::args().collect();

    // Allow piping a JSON array of args via stdin using `-`
    if args.len() == 2 && args[1] == "-" {
        let mut input = String::new();
        if std::io::stdin().read_to_string(&mut input).is_ok() {
            if let Ok(parsed_args) = serde_json::from_str::<Vec<String>>(&input) {
                let prog = args[0].clone();
                args = std::iter::once(prog).chain(parsed_args).collect();
            }
        }
    }

    // Strip global flags: -p/--profile and -o/--output
    let mut profile_name = None;
    let mut output_format = "text".to_string();
    let mut cmd_args = vec![args[0].clone()];

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-p" | "--profile" if i + 1 < args.len() => {
                profile_name = Some(args[i + 1].clone());
                i += 2;
            }
            s if s.starts_with("--profile=") => {
                profile_name = Some(s["--profile=".len()..].to_string());
                i += 1;
            }
            s if s.starts_with("-p=") => {
                profile_name = Some(s["-p=".len()..].to_string());
                i += 1;
            }
            "-o" | "--output" if i + 1 < args.len() => {
                output_format = args[i + 1].clone();
                i += 2;
            }
            s if s.starts_with("--output=") => {
                output_format = s["--output=".len()..].to_string();
                i += 1;
            }
            s if s.starts_with("-o=") => {
                output_format = s["-o=".len()..].to_string();
                i += 1;
            }
            _ => {
                cmd_args.push(args[i].clone());
                i += 1;
            }
        }
    }

    if cmd_args.len() < 2 {
        print_usage();
        return;
    }

    let result = match cmd_args[1].as_str() {
        "config" | "cfg" => handle_config(&cmd_args[2..]),
        "jira" | "j" => handle_jira(&cmd_args[2..], profile_name.as_deref(), &output_format),
        "confluence" | "conf" | "c" => {
            handle_confluence(&cmd_args[2..], profile_name.as_deref(), &output_format)
        }
        "bitbucket" | "bb" => {
            handle_bitbucket(&cmd_args[2..], profile_name.as_deref(), &output_format)
        }
        "alert" => handle_alert(&cmd_args[2..], profile_name.as_deref(), &output_format),
        "version" => {
            println!("acli v{}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        cmd => {
            eprintln!("Unknown command: {}", cmd);
            print_usage();
            std::process::exit(1);
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn print_usage() {
    println!("ACLI - Atlassian CLI (Rust Port)");
    println!("\nUsage: acli [global-flags] <command> [arguments]");
    println!("\nGlobal Flags:");
    println!("  -p, --profile <name>   Configuration profile to use");
    println!("  -o, --output <format>  Output format: text (default) or json");
    println!("\nCommands:");
    println!("  config              Manage configuration profiles");
    println!("  jira, j             Interact with Jira Cloud");
    println!("  confluence, conf, c Interact with Confluence");
    println!("  bitbucket, bb       Interact with Bitbucket Cloud");
    println!("  alert               Interact with Service Manager Alerts");
    println!("  version             Print version information");
}

// ---------------------------------------------------------------------------
// config
// ---------------------------------------------------------------------------

fn handle_config(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        println!("Config Commands:");
        println!("  setup <profile>       Create or update a profile interactively");
        println!("  list                  List all profiles");
        println!("  show [profile]        Show profile details (token masked)");
        println!("  delete <profile>      Delete a profile");
        println!("  set-default <profile> Set default profile");
        return Ok(());
    }

    match args[0].as_str() {
        "setup" => {
            if args.len() < 2 {
                return Err("Usage: acli config setup <profile-name>".to_string());
            }
            config::run_setup(&args[1])
        }
        "list" | "ls" => config::run_list(),
        "show" => config::run_show(args.get(1).map(|s| s.as_str())),
        "delete" | "rm" => {
            if args.len() < 2 {
                return Err("Usage: acli config delete <profile-name>".to_string());
            }
            config::run_delete(&args[1])
        }
        "set-default" => {
            if args.len() < 2 {
                return Err("Usage: acli config set-default <profile-name>".to_string());
            }
            config::run_set_default(&args[1])
        }
        sub => Err(format!("Unknown config subcommand: {}", sub)),
    }
}

// ---------------------------------------------------------------------------
// jira — top-level routing
// ---------------------------------------------------------------------------

fn handle_jira(args: &[String], profile: Option<&str>, output: &str) -> Result<(), String> {
    if args.is_empty() {
        println!("Jira Commands:");
        println!(
            "  issue, i   Manage issues (list, get, create, edit, delete, assign, transition)"
        );
        println!("  board, b   Manage boards (list, sprints)");
        println!("  sprint, s  Sprint commands (issues)");
        println!("  epic, e    Epic commands (issues)");
        return Ok(());
    }

    match args[0].as_str() {
        "issue" | "i" => handle_jira_issue(&args[1..], profile, output),
        "board" | "b" => handle_jira_board(&args[1..], profile, output),
        "sprint" | "s" => handle_jira_sprint(&args[1..], profile, output),
        "epic" | "e" => handle_jira_epic(&args[1..], profile, output),
        res => Err(format!("Unknown jira resource: {}", res)),
    }
}

// ---------------------------------------------------------------------------
// jira issue
// ---------------------------------------------------------------------------

fn handle_jira_issue(args: &[String], profile: Option<&str>, output: &str) -> Result<(), String> {
    if args.is_empty() {
        println!("Jira Issue Commands:");
        println!("  list                  List issues (JQL / filters)");
        println!("  get <key>             Get issue details");
        println!("  create                Create a new issue");
        println!("  edit <key>            Edit an issue");
        println!("  delete <key>          Delete an issue");
        println!("  assign <key> <id>     Assign an issue (use 'none' to unassign)");
        println!("  transition <key>      Transition an issue to a new status");
        println!("  transitions <key>     List available transitions");
        println!("  comment               Manage comments");
        println!("  worklog               Manage work logs");
        println!("  attach <key> <file>   Upload a file attachment");
        return Ok(());
    }

    match args[0].as_str() {
        "list" | "ls" => issue_list(&args[1..], profile, output),
        "get" => issue_get(&args[1..], profile, output),
        "create" => issue_create(&args[1..], profile, output),
        "edit" => issue_edit(&args[1..], profile, output),
        "delete" | "rm" => issue_delete(&args[1..], profile, output),
        "assign" => issue_assign(&args[1..], profile),
        "transition" => issue_transition(&args[1..], profile, output),
        "transitions" => issue_transitions(&args[1..], profile, output),
        "comment" | "c" => handle_issue_comment(&args[1..], profile, output),
        "worklog" | "wl" => handle_issue_worklog(&args[1..], profile, output),
        "attach" => issue_attach(&args[1..], profile),
        action => Err(format!("Unknown issue action: {}", action)),
    }
}

fn issue_list(args: &[String], profile: Option<&str>, output: &str) -> Result<(), String> {
    let cfg = Config::load()?;
    let prof = cfg.get_profile(profile)?;
    let client = Client::new(prof.clone());

    let mut jql_flag = None;
    let mut project_flag = None;
    let mut assignee_flag = None;
    let mut status_flag = None;
    let mut max_results = 50i32;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--jql" if i + 1 < args.len() => {
                jql_flag = Some(args[i + 1].clone());
                i += 2;
            }
            "--project" if i + 1 < args.len() => {
                project_flag = Some(args[i + 1].clone());
                i += 2;
            }
            "--assignee" if i + 1 < args.len() => {
                assignee_flag = Some(args[i + 1].clone());
                i += 2;
            }
            "--status" if i + 1 < args.len() => {
                status_flag = Some(args[i + 1].clone());
                i += 2;
            }
            "--max-results" if i + 1 < args.len() => {
                max_results = args[i + 1].parse().unwrap_or(50);
                i += 2;
            }
            _ => i += 1,
        }
    }

    let project = project_flag.or_else(|| prof.defaults.and_then(|d| d.project));
    let jql = match jql_flag {
        Some(j) => j,
        None => {
            let mut clauses = Vec::new();
            if let Some(ref p) = project {
                clauses.push(format!("project = {}", p));
            }
            if let Some(ref a) = assignee_flag {
                clauses.push(format!("assignee = \"{}\"", a));
            }
            if let Some(ref s) = status_flag {
                clauses.push(format!("status = \"{}\"", s));
            }
            if clauses.is_empty() {
                "created >= -30d order by created DESC".to_string()
            } else {
                format!("{} order by created DESC", clauses.join(" AND "))
            }
        }
    };

    let fields = ["summary", "issuetype", "status", "priority", "assignee"];
    let results = jira::search_jql(&client, &jql, 0, max_results, &fields)?;

    if output == "json" {
        println!("{}", serde_json::to_string_pretty(&results).unwrap());
        return Ok(());
    }

    println!(
        "{:<15} {:<10} {:<15} {:<10} {:<25} SUMMARY",
        "KEY", "TYPE", "STATUS", "PRIORITY", "ASSIGNEE"
    );
    for issue in results.issues {
        let type_name = issue.fields.issuetype.map(|t| t.name).unwrap_or_default();
        let status_name = issue.fields.status.map(|s| s.name).unwrap_or_default();
        let priority_name = issue.fields.priority.map(|p| p.name).unwrap_or_default();
        let assignee_name = issue
            .fields
            .assignee
            .map(|a| a.display_name)
            .unwrap_or_else(|| "Unassigned".to_string());
        println!(
            "{:<15} {:<10} {:<15} {:<10} {:<25} {}",
            issue.key, type_name, status_name, priority_name, assignee_name, issue.fields.summary
        );
    }
    Ok(())
}

fn issue_get(args: &[String], profile: Option<&str>, output: &str) -> Result<(), String> {
    if args.is_empty() {
        return Err("Usage: acli jira issue get <issue-key>".to_string());
    }
    let key = &args[0];
    let cfg = Config::load()?;
    let client = Client::new(cfg.get_profile(profile)?);
    let issue = jira::get_issue(&client, key)?;

    if output == "json" {
        println!("{}", serde_json::to_string_pretty(&issue).unwrap());
        return Ok(());
    }

    let f = issue.fields;
    let type_name = f.issuetype.map(|t| t.name).unwrap_or_default();
    let status_name = f.status.map(|s| s.name).unwrap_or_default();
    let priority_name = f.priority.map(|p| p.name).unwrap_or_default();
    let assignee_name = f
        .assignee
        .map(|a| a.display_name)
        .unwrap_or_else(|| "Unassigned".to_string());
    let reporter_name = f
        .reporter
        .map(|r| r.display_name)
        .unwrap_or_else(|| "None".to_string());
    let description = f
        .description
        .map(|d| jira::render_adf(&d))
        .unwrap_or_default();
    let components: Vec<String> = f.components.into_iter().map(|c| c.name).collect();
    let fix_versions: Vec<String> = f.fix_versions.into_iter().map(|v| v.name).collect();

    println!("Key:          {}", issue.key);
    println!("Summary:      {}", f.summary);
    println!("Status:       {}", status_name);
    println!("Type:         {}", type_name);
    println!("Priority:     {}", priority_name);
    println!("Assignee:     {}", assignee_name);
    println!("Reporter:     {}", reporter_name);
    println!("Created:      {}", f.created.unwrap_or_default());
    println!("Updated:      {}", f.updated.unwrap_or_default());
    println!("Labels:       {}", f.labels.join(", "));
    println!("Components:   {}", components.join(", "));
    println!("Fix Versions: {}", fix_versions.join(", "));
    println!("Description:\n{}", description);
    Ok(())
}

fn issue_create(args: &[String], profile: Option<&str>, output: &str) -> Result<(), String> {
    let cfg = Config::load()?;
    let prof = cfg.get_profile(profile)?;
    let client = Client::new(prof.clone());

    let mut project = prof.defaults.and_then(|d| d.project);
    let mut summary = None;
    let mut issue_type = "Task".to_string();
    let mut description = None;
    let mut priority = None;
    let mut assignee = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--project" if i + 1 < args.len() => {
                project = Some(args[i + 1].clone());
                i += 2;
            }
            "--summary" if i + 1 < args.len() => {
                summary = Some(args[i + 1].clone());
                i += 2;
            }
            "--type" if i + 1 < args.len() => {
                issue_type = args[i + 1].clone();
                i += 2;
            }
            "--description" if i + 1 < args.len() => {
                description = Some(args[i + 1].clone());
                i += 2;
            }
            "--priority" if i + 1 < args.len() => {
                priority = Some(args[i + 1].clone());
                i += 2;
            }
            "--assignee" if i + 1 < args.len() => {
                assignee = Some(args[i + 1].clone());
                i += 2;
            }
            _ => i += 1,
        }
    }

    let project = project.ok_or("--project is required (or set a profile default)")?;
    let summary = summary.ok_or("--summary is required")?;

    let created = jira::create_issue(
        &client,
        &project,
        &summary,
        &issue_type,
        description.as_deref(),
        priority.as_deref(),
    )?;

    // Assign after creation if requested (create doesn't support assignee directly)
    if let Some(ref account_id) = assignee {
        jira::assign_issue(&client, &created.key, Some(account_id))?;
    }

    if output == "json" {
        println!("{}", serde_json::to_string_pretty(&created).unwrap());
    } else {
        println!("Created issue: {}", created.key);
    }
    Ok(())
}

fn issue_edit(args: &[String], profile: Option<&str>, output: &str) -> Result<(), String> {
    if args.is_empty() {
        return Err("Usage: acli jira issue edit <key> [--summary <s>] [--description <d>] [--priority <p>]".to_string());
    }
    let key = &args[0];

    let mut summary = None;
    let mut description = None;
    let mut priority = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--summary" if i + 1 < args.len() => {
                summary = Some(args[i + 1].as_str());
                i += 2;
            }
            "--description" if i + 1 < args.len() => {
                description = Some(args[i + 1].as_str());
                i += 2;
            }
            "--priority" if i + 1 < args.len() => {
                priority = Some(args[i + 1].as_str());
                i += 2;
            }
            _ => i += 1,
        }
    }

    if summary.is_none() && description.is_none() && priority.is_none() {
        return Err(
            "At least one field (--summary, --description, --priority) is required".to_string(),
        );
    }

    let cfg = Config::load()?;
    let client = Client::new(cfg.get_profile(profile)?);
    jira::edit_issue(&client, key, summary, description, priority)?;

    if output == "json" {
        println!("{{\"key\":\"{}\",\"updated\":true}}", key);
    } else {
        println!("Issue {} updated", key);
    }
    Ok(())
}

fn issue_delete(args: &[String], profile: Option<&str>, output: &str) -> Result<(), String> {
    if args.is_empty() {
        return Err("Usage: acli jira issue delete <key> [--delete-subtasks]".to_string());
    }
    let key = &args[0];
    let delete_subtasks = args.iter().any(|a| a == "--delete-subtasks");

    let cfg = Config::load()?;
    let client = Client::new(cfg.get_profile(profile)?);
    jira::delete_issue(&client, key, delete_subtasks)?;

    if output != "json" {
        println!("Issue {} deleted", key);
    }
    Ok(())
}

fn issue_assign(args: &[String], profile: Option<&str>) -> Result<(), String> {
    if args.len() < 2 {
        return Err("Usage: acli jira issue assign <key> <account-id|none>".to_string());
    }
    let key = &args[0];
    let account_id = &args[1];
    let resolved = if account_id == "none" || account_id == "-" {
        None
    } else {
        Some(account_id.as_str())
    };

    let cfg = Config::load()?;
    let client = Client::new(cfg.get_profile(profile)?);
    jira::assign_issue(&client, key, resolved)?;

    match resolved {
        Some(id) => println!("Issue {} assigned to {}", key, id),
        None => println!("Issue {} unassigned", key),
    }
    Ok(())
}

fn issue_transition(args: &[String], profile: Option<&str>, output: &str) -> Result<(), String> {
    if args.is_empty() {
        return Err(
            "Usage: acli jira issue transition <key> --id <id> | --status <name>".to_string(),
        );
    }
    let key = &args[0];

    let mut transition_id = None;
    let mut status_name = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--id" if i + 1 < args.len() => {
                transition_id = Some(args[i + 1].clone());
                i += 2;
            }
            "--status" if i + 1 < args.len() => {
                status_name = Some(args[i + 1].clone());
                i += 2;
            }
            _ => i += 1,
        }
    }

    let cfg = Config::load()?;
    let client = Client::new(cfg.get_profile(profile)?);

    // Resolve by status name if no ID given
    let tid = if let Some(id) = transition_id {
        id
    } else if let Some(ref name) = status_name {
        let transitions = jira::get_transitions(&client, key)?;
        transitions
            .into_iter()
            .find(|t| t.name.eq_ignore_ascii_case(name))
            .map(|t| t.id)
            .ok_or_else(|| format!("No transition found matching status '{}'", name))?
    } else {
        return Err("Provide --id <transition-id> or --status <name>".to_string());
    };

    jira::do_transition(&client, key, &tid)?;

    if output != "json" {
        println!("Issue {} transitioned (transition {})", key, tid);
    }
    Ok(())
}

fn issue_transitions(args: &[String], profile: Option<&str>, output: &str) -> Result<(), String> {
    if args.is_empty() {
        return Err("Usage: acli jira issue transitions <key>".to_string());
    }
    let key = &args[0];
    let cfg = Config::load()?;
    let client = Client::new(cfg.get_profile(profile)?);
    let transitions = jira::get_transitions(&client, key)?;

    if output == "json" {
        println!("{}", serde_json::to_string_pretty(&transitions).unwrap());
        return Ok(());
    }

    println!("{:<8} NAME", "ID");
    for t in &transitions {
        println!("{:<8} {}", t.id, t.name);
    }
    Ok(())
}

fn issue_attach(args: &[String], profile: Option<&str>) -> Result<(), String> {
    if args.len() < 2 {
        return Err("Usage: acli jira issue attach <key> <file-path>".to_string());
    }
    let key = &args[0];
    let file_path = &args[1];
    let cfg = Config::load()?;
    let client = Client::new(cfg.get_profile(profile)?);
    let resp = jira::attach_file(&client, key, file_path)?;
    println!("Attachment uploaded to {}: {}", key, resp);
    Ok(())
}

// ---------------------------------------------------------------------------
// jira issue comment
// ---------------------------------------------------------------------------

fn handle_issue_comment(
    args: &[String],
    profile: Option<&str>,
    output: &str,
) -> Result<(), String> {
    if args.is_empty() {
        println!("Issue Comment Commands:");
        println!("  list <key>              List comments");
        println!("  add  <key> --body <text> Add a comment");
        println!("  delete <key> <id>       Delete a comment");
        return Ok(());
    }

    match args[0].as_str() {
        "list" | "ls" => {
            let key = args
                .get(1)
                .ok_or("Usage: acli jira issue comment list <key>")?;
            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            let comments = jira::list_comments(&client, key)?;

            if output == "json" {
                println!("{}", serde_json::to_string_pretty(&comments).unwrap());
                return Ok(());
            }

            println!("{:<12} {:<25} {:<22} BODY", "ID", "AUTHOR", "CREATED");
            for c in &comments {
                let author = c
                    .author
                    .as_ref()
                    .map(|a| a.display_name.as_str())
                    .unwrap_or("");
                let created = c.created.as_deref().unwrap_or("");
                let body_text = c.body.as_ref().map(jira::render_adf).unwrap_or_default();
                let preview: String = body_text.chars().take(60).collect();
                println!("{:<12} {:<25} {:<22} {}", c.id, author, created, preview);
            }
            Ok(())
        }
        "add" => {
            let key = args
                .get(1)
                .ok_or("Usage: acli jira issue comment add <key> --body <text>")?;
            let mut body_text = None;
            let mut i = 2;
            while i < args.len() {
                if args[i] == "--body" && i + 1 < args.len() {
                    body_text = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    i += 1;
                }
            }
            let body_text = body_text.ok_or("--body <text> is required")?;
            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            let comment = jira::add_comment(&client, key, &body_text)?;

            if output == "json" {
                println!("{}", serde_json::to_string_pretty(&comment).unwrap());
            } else {
                println!("Comment {} added to {}", comment.id, key);
            }
            Ok(())
        }
        "delete" | "rm" => {
            if args.len() < 3 {
                return Err("Usage: acli jira issue comment delete <key> <comment-id>".to_string());
            }
            let key = &args[1];
            let comment_id = &args[2];
            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            jira::delete_comment(&client, key, comment_id)?;
            println!("Comment {} deleted from {}", comment_id, key);
            Ok(())
        }
        sub => Err(format!("Unknown comment subcommand: {}", sub)),
    }
}

// ---------------------------------------------------------------------------
// jira issue worklog
// ---------------------------------------------------------------------------

fn handle_issue_worklog(
    args: &[String],
    profile: Option<&str>,
    output: &str,
) -> Result<(), String> {
    if args.is_empty() {
        println!("Issue Worklog Commands:");
        println!("  list <key>                           List worklogs");
        println!("  add  <key> --time-spent <t> [--comment <c>]  Add a worklog");
        println!("  delete <key> <id>                    Delete a worklog");
        return Ok(());
    }

    match args[0].as_str() {
        "list" | "ls" => {
            let key = args
                .get(1)
                .ok_or("Usage: acli jira issue worklog list <key>")?;
            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            let worklogs = jira::list_worklogs(&client, key)?;

            if output == "json" {
                println!("{}", serde_json::to_string_pretty(&worklogs).unwrap());
                return Ok(());
            }

            println!("{:<12} {:<25} {:<12} COMMENT", "ID", "AUTHOR", "TIME SPENT");
            for w in &worklogs {
                let author = w
                    .author
                    .as_ref()
                    .map(|a| a.display_name.as_str())
                    .unwrap_or("");
                let time = w.time_spent.as_deref().unwrap_or("");
                let comment_text = w.comment.as_ref().map(jira::render_adf).unwrap_or_default();
                let preview: String = comment_text.chars().take(50).collect();
                println!("{:<12} {:<25} {:<12} {}", w.id, author, time, preview);
            }
            Ok(())
        }
        "add" => {
            let key = args
                .get(1)
                .ok_or("Usage: acli jira issue worklog add <key> --time-spent <t>")?;
            let mut time_spent = None;
            let mut comment = None;
            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--time-spent" if i + 1 < args.len() => {
                        time_spent = Some(args[i + 1].clone());
                        i += 2;
                    }
                    "--comment" if i + 1 < args.len() => {
                        comment = Some(args[i + 1].clone());
                        i += 2;
                    }
                    _ => i += 1,
                }
            }
            let time_spent =
                time_spent.ok_or("--time-spent <duration> is required (e.g. 2h, 30m)")?;
            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            let worklog = jira::add_worklog(&client, key, &time_spent, comment.as_deref())?;

            if output == "json" {
                println!("{}", serde_json::to_string_pretty(&worklog).unwrap());
            } else {
                println!("Worklog {} added to {} ({})", worklog.id, key, time_spent);
            }
            Ok(())
        }
        "delete" | "rm" => {
            if args.len() < 3 {
                return Err("Usage: acli jira issue worklog delete <key> <worklog-id>".to_string());
            }
            let key = &args[1];
            let worklog_id = &args[2];
            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            jira::delete_worklog(&client, key, worklog_id)?;
            println!("Worklog {} deleted from {}", worklog_id, key);
            Ok(())
        }
        sub => Err(format!("Unknown worklog subcommand: {}", sub)),
    }
}

// ---------------------------------------------------------------------------
// jira board
// ---------------------------------------------------------------------------

fn handle_jira_board(args: &[String], profile: Option<&str>, output: &str) -> Result<(), String> {
    if args.is_empty() {
        println!("Board Commands:");
        println!("  list              List boards");
        println!("  sprints <id>      List sprints for a board");
        return Ok(());
    }

    match args[0].as_str() {
        "list" | "ls" => {
            let mut project = None;
            let mut max_results = 50i32;
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--project" if i + 1 < args.len() => {
                        project = Some(args[i + 1].as_str());
                        i += 2;
                    }
                    "--max-results" if i + 1 < args.len() => {
                        max_results = args[i + 1].parse().unwrap_or(50);
                        i += 2;
                    }
                    _ => i += 1,
                }
            }
            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            let boards = agile::list_boards(&client, 0, max_results, project)?;

            if output == "json" {
                println!("{}", serde_json::to_string_pretty(&boards).unwrap());
                return Ok(());
            }

            println!("{:<6} {:<30} {:<10} PROJECT", "ID", "NAME", "TYPE");
            for b in &boards.values {
                let proj = b
                    .location
                    .as_ref()
                    .map(|l| l.project_key.as_str())
                    .unwrap_or("");
                println!("{:<6} {:<30} {:<10} {}", b.id, b.name, b.board_type, proj);
            }
            Ok(())
        }
        "sprints" => {
            let board_id: i32 = args
                .get(1)
                .ok_or("Usage: acli jira board sprints <board-id>")?
                .parse()
                .map_err(|_| "Board ID must be a number".to_string())?;

            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            let sprints = agile::get_board_sprints(&client, board_id, 0, 50)?;

            if output == "json" {
                println!("{}", serde_json::to_string_pretty(&sprints).unwrap());
                return Ok(());
            }

            println!(
                "{:<6} {:<30} {:<10} {:<26} END",
                "ID", "NAME", "STATE", "START"
            );
            for s in &sprints.values {
                println!(
                    "{:<6} {:<30} {:<10} {:<26} {}",
                    s.id,
                    s.name,
                    s.state,
                    s.start_date.as_deref().unwrap_or(""),
                    s.end_date.as_deref().unwrap_or("")
                );
            }
            Ok(())
        }
        action => Err(format!("Unknown board action: {}", action)),
    }
}

// ---------------------------------------------------------------------------
// jira sprint
// ---------------------------------------------------------------------------

fn handle_jira_sprint(args: &[String], profile: Option<&str>, output: &str) -> Result<(), String> {
    if args.is_empty() {
        println!("Sprint Commands:");
        println!("  issues <sprint-id>   List issues in a sprint");
        return Ok(());
    }

    match args[0].as_str() {
        "issues" => {
            let sprint_id: i32 = args
                .get(1)
                .ok_or("Usage: acli jira sprint issues <sprint-id>")?
                .parse()
                .map_err(|_| "Sprint ID must be a number".to_string())?;

            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            let results = agile::get_sprint_issues(&client, sprint_id, 0, 50)?;

            if output == "json" {
                println!("{}", serde_json::to_string_pretty(&results).unwrap());
                return Ok(());
            }

            println!(
                "{:<15} {:<10} {:<15} {:<10} SUMMARY",
                "KEY", "TYPE", "STATUS", "PRIORITY"
            );
            for issue in results.issues {
                let type_name = issue.fields.issuetype.map(|t| t.name).unwrap_or_default();
                let status_name = issue.fields.status.map(|s| s.name).unwrap_or_default();
                let priority_name = issue.fields.priority.map(|p| p.name).unwrap_or_default();
                println!(
                    "{:<15} {:<10} {:<15} {:<10} {}",
                    issue.key, type_name, status_name, priority_name, issue.fields.summary
                );
            }
            Ok(())
        }
        action => Err(format!("Unknown sprint action: {}", action)),
    }
}

// ---------------------------------------------------------------------------
// jira epic
// ---------------------------------------------------------------------------

fn handle_jira_epic(args: &[String], profile: Option<&str>, output: &str) -> Result<(), String> {
    if args.is_empty() {
        println!("Epic Commands:");
        println!("  issues <epic-key>   List issues in an epic");
        return Ok(());
    }

    match args[0].as_str() {
        "issues" => {
            let epic_key = args
                .get(1)
                .ok_or("Usage: acli jira epic issues <epic-key>")?;
            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            let results = agile::get_epic_issues(&client, epic_key, 0, 50)?;

            if output == "json" {
                println!("{}", serde_json::to_string_pretty(&results).unwrap());
                return Ok(());
            }

            println!(
                "{:<15} {:<10} {:<15} {:<10} SUMMARY",
                "KEY", "TYPE", "STATUS", "PRIORITY"
            );
            for issue in results.issues {
                let type_name = issue.fields.issuetype.map(|t| t.name).unwrap_or_default();
                let status_name = issue.fields.status.map(|s| s.name).unwrap_or_default();
                let priority_name = issue.fields.priority.map(|p| p.name).unwrap_or_default();
                println!(
                    "{:<15} {:<10} {:<15} {:<10} {}",
                    issue.key, type_name, status_name, priority_name, issue.fields.summary
                );
            }
            Ok(())
        }
        action => Err(format!("Unknown epic action: {}", action)),
    }
}

// ---------------------------------------------------------------------------
// confluence — top-level routing
// ---------------------------------------------------------------------------

fn handle_confluence(args: &[String], profile: Option<&str>, output: &str) -> Result<(), String> {
    if args.is_empty() {
        println!("Confluence Commands:");
        println!("  space, s   Manage spaces (list, get, create, pages)");
        println!("  page, p    Manage pages (list, get, create, update, delete)");
        return Ok(());
    }

    match args[0].as_str() {
        "space" | "s" => handle_conf_space(&args[1..], profile, output),
        "page" | "p" => handle_conf_page(&args[1..], profile, output),
        res => Err(format!("Unknown confluence resource: {}", res)),
    }
}

// ---------------------------------------------------------------------------
// confluence space
// ---------------------------------------------------------------------------

fn handle_conf_space(args: &[String], profile: Option<&str>, output: &str) -> Result<(), String> {
    if args.is_empty() {
        println!("Space Commands:");
        println!("  list                           List spaces");
        println!("  get <id>                       Get space details");
        println!("  create --name <n> [--key <k>]  Create a space");
        println!("  pages <id> [--title <t>]       List pages in a space");
        return Ok(());
    }

    match args[0].as_str() {
        "list" | "ls" => {
            let mut limit = 50i32;
            let mut space_type = None;
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--limit" if i + 1 < args.len() => {
                        limit = args[i + 1].parse().unwrap_or(50);
                        i += 2;
                    }
                    "--type" if i + 1 < args.len() => {
                        space_type = Some(args[i + 1].as_str());
                        i += 2;
                    }
                    _ => i += 1,
                }
            }
            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            let spaces = confluence::list_spaces(&client, limit, space_type)?;

            if output == "json" {
                println!("{}", serde_json::to_string_pretty(&spaces).unwrap());
                return Ok(());
            }

            println!("{:<20} {:<15} {:<12} NAME", "ID", "KEY", "TYPE");
            for s in &spaces.results {
                println!(
                    "{:<20} {:<15} {:<12} {}",
                    s.id,
                    s.key.as_deref().unwrap_or(""),
                    s.space_type.as_deref().unwrap_or(""),
                    s.name
                );
            }
            Ok(())
        }
        "get" => {
            let id = args.get(1).ok_or("Usage: acli confluence space get <id>")?;
            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            let space = confluence::get_space(&client, id)?;

            if output == "json" {
                println!("{}", serde_json::to_string_pretty(&space).unwrap());
                return Ok(());
            }

            println!("ID:     {}", space.id);
            println!("Key:    {}", space.key.as_deref().unwrap_or(""));
            println!("Name:   {}", space.name);
            println!("Type:   {}", space.space_type.as_deref().unwrap_or(""));
            println!("Status: {}", space.status.as_deref().unwrap_or(""));
            Ok(())
        }
        "create" => {
            let mut name = None;
            let mut key = None;
            let mut description = None;
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--name" if i + 1 < args.len() => {
                        name = Some(args[i + 1].as_str());
                        i += 2;
                    }
                    "--key" if i + 1 < args.len() => {
                        key = Some(args[i + 1].as_str());
                        i += 2;
                    }
                    "--description" if i + 1 < args.len() => {
                        description = Some(args[i + 1].as_str());
                        i += 2;
                    }
                    _ => i += 1,
                }
            }
            let name = name.ok_or("--name is required")?;
            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            let space = confluence::create_space(&client, name, key, description)?;

            if output == "json" {
                println!("{}", serde_json::to_string_pretty(&space).unwrap());
            } else {
                println!("Created space: {} ({})", space.name, space.id);
            }
            Ok(())
        }
        "pages" => {
            let space_id = args
                .get(1)
                .ok_or("Usage: acli confluence space pages <space-id>")?;
            let mut title = None;
            let mut limit = 50i32;
            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--title" if i + 1 < args.len() => {
                        title = Some(args[i + 1].as_str());
                        i += 2;
                    }
                    "--limit" if i + 1 < args.len() => {
                        limit = args[i + 1].parse().unwrap_or(50);
                        i += 2;
                    }
                    _ => i += 1,
                }
            }
            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            let pages = confluence::list_space_pages(&client, space_id, title, limit)?;

            if output == "json" {
                println!("{}", serde_json::to_string_pretty(&pages).unwrap());
                return Ok(());
            }

            println!("{:<20} {:<12} TITLE", "ID", "STATUS");
            for p in &pages.results {
                println!(
                    "{:<20} {:<12} {}",
                    p.id,
                    p.status.as_deref().unwrap_or(""),
                    p.title
                );
            }
            Ok(())
        }
        action => Err(format!("Unknown space action: {}", action)),
    }
}

// ---------------------------------------------------------------------------
// confluence page
// ---------------------------------------------------------------------------

fn handle_conf_page(args: &[String], profile: Option<&str>, output: &str) -> Result<(), String> {
    if args.is_empty() {
        println!("Page Commands:");
        println!("  list [--space-id <id>] [--title <t>]              List pages");
        println!("  get <id>                                           Get page details");
        println!("  create --space-id <id> --title <t> [--body <html>] Create a page");
        println!("  update <id> --title <t> --version <n> [--body <h>] Update a page");
        println!("  delete <id>                                        Delete a page");
        return Ok(());
    }

    match args[0].as_str() {
        "list" | "ls" => {
            let mut space_id = None;
            let mut title = None;
            let mut limit = 50i32;
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--space-id" if i + 1 < args.len() => {
                        space_id = Some(args[i + 1].as_str());
                        i += 2;
                    }
                    "--title" if i + 1 < args.len() => {
                        title = Some(args[i + 1].as_str());
                        i += 2;
                    }
                    "--limit" if i + 1 < args.len() => {
                        limit = args[i + 1].parse().unwrap_or(50);
                        i += 2;
                    }
                    _ => i += 1,
                }
            }
            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            let pages = confluence::list_pages(&client, space_id, title, limit)?;

            if output == "json" {
                println!("{}", serde_json::to_string_pretty(&pages).unwrap());
                return Ok(());
            }

            println!("{:<20} {:<12} TITLE", "ID", "STATUS");
            for p in &pages.results {
                println!(
                    "{:<20} {:<12} {}",
                    p.id,
                    p.status.as_deref().unwrap_or(""),
                    p.title
                );
            }
            Ok(())
        }
        "get" => {
            let id = args.get(1).ok_or("Usage: acli confluence page get <id>")?;
            let with_body = args.iter().any(|a| a == "--body");
            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            let page = confluence::get_page(&client, id, with_body)?;

            if output == "json" {
                println!("{}", serde_json::to_string_pretty(&page).unwrap());
                return Ok(());
            }

            println!("ID:       {}", page.id);
            println!("Title:    {}", page.title);
            println!("Space:    {}", page.space_id.as_deref().unwrap_or(""));
            println!("Status:   {}", page.status.as_deref().unwrap_or(""));
            println!("Author:   {}", page.author_id.as_deref().unwrap_or(""));
            println!("Created:  {}", page.created_at.as_deref().unwrap_or(""));
            if let Some(v) = &page.version {
                println!("Version:  {}", v.number);
            }
            if let Some(body) = &page.body {
                if let Some(html) = &body.value {
                    println!("\nContent:\n{}", confluence::render_storage(html));
                }
            }
            Ok(())
        }
        "create" => {
            let mut space_id = None;
            let mut title = None;
            let mut body_html = None;
            let mut parent_id = None;
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--space-id" if i + 1 < args.len() => {
                        space_id = Some(args[i + 1].as_str());
                        i += 2;
                    }
                    "--title" if i + 1 < args.len() => {
                        title = Some(args[i + 1].as_str());
                        i += 2;
                    }
                    "--body" if i + 1 < args.len() => {
                        body_html = Some(args[i + 1].as_str());
                        i += 2;
                    }
                    "--parent-id" if i + 1 < args.len() => {
                        parent_id = Some(args[i + 1].as_str());
                        i += 2;
                    }
                    _ => i += 1,
                }
            }
            let space_id = space_id.ok_or("--space-id is required")?;
            let title = title.ok_or("--title is required")?;
            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            let created = confluence::create_page(&client, space_id, title, body_html, parent_id)?;

            if output == "json" {
                println!("{}", serde_json::to_string_pretty(&created).unwrap());
            } else {
                println!("Created page: {} (id: {})", created.title, created.id);
            }
            Ok(())
        }
        "update" => {
            let id = args
                .get(1)
                .ok_or("Usage: acli confluence page update <id> --title <t> --version <n>")?;
            let mut title = None;
            let mut version_number = None;
            let mut body_html = None;
            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--title" if i + 1 < args.len() => {
                        title = Some(args[i + 1].as_str());
                        i += 2;
                    }
                    "--version" if i + 1 < args.len() => {
                        version_number = args[i + 1].parse().ok();
                        i += 2;
                    }
                    "--body" if i + 1 < args.len() => {
                        body_html = Some(args[i + 1].as_str());
                        i += 2;
                    }
                    _ => i += 1,
                }
            }
            let title = title.ok_or("--title is required")?;
            let version_number = version_number.ok_or("--version <number> is required")?;
            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            let page = confluence::update_page(&client, id, title, version_number, body_html)?;

            if output == "json" {
                println!("{}", serde_json::to_string_pretty(&page).unwrap());
            } else {
                println!("Updated page: {} (version {})", page.title, version_number);
            }
            Ok(())
        }
        "delete" | "rm" => {
            let id = args
                .get(1)
                .ok_or("Usage: acli confluence page delete <id>")?;
            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            confluence::delete_page(&client, id)?;
            println!("Page {} deleted", id);
            Ok(())
        }
        action => Err(format!("Unknown page action: {}", action)),
    }
}

// ---------------------------------------------------------------------------
// bitbucket — top-level routing
// ---------------------------------------------------------------------------

fn handle_bitbucket(args: &[String], profile: Option<&str>, output: &str) -> Result<(), String> {
    if args.is_empty() {
        println!("Bitbucket Commands:");
        println!("  repo, r      Manage repositories (list, get, create, delete)");
        println!(
            "  pr           Manage pull requests (list, get, create, approve, merge, decline)"
        );
        println!("  pipeline, p  Manage pipelines (list, get, run, stop, steps, log)");
        return Ok(());
    }

    match args[0].as_str() {
        "repo" | "r" => handle_bb_repo(&args[1..], profile, output),
        "pr" => handle_bb_pr(&args[1..], profile, output),
        "pipeline" | "p" | "pipe" => handle_bb_pipeline(&args[1..], profile, output),
        res => Err(format!("Unknown bitbucket resource: {}", res)),
    }
}

// ---------------------------------------------------------------------------
// bitbucket repo
// ---------------------------------------------------------------------------

fn handle_bb_repo(args: &[String], profile: Option<&str>, output: &str) -> Result<(), String> {
    if args.is_empty() {
        println!("Repo Commands:");
        println!("  list <workspace>                  List repositories");
        println!("  get <workspace> <slug>            Get repository details");
        println!("  create <workspace> <slug> --name  Create a repository");
        println!("  delete <workspace> <slug>         Delete a repository");
        return Ok(());
    }

    match args[0].as_str() {
        "list" | "ls" => {
            let workspace = args.get(1).ok_or("Usage: acli bb repo list <workspace>")?;
            let mut role = None;
            let mut query = None;
            let mut page_len = 50i32;
            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--role" if i + 1 < args.len() => {
                        role = Some(args[i + 1].as_str());
                        i += 2;
                    }
                    "--query" if i + 1 < args.len() => {
                        query = Some(args[i + 1].as_str());
                        i += 2;
                    }
                    "--limit" if i + 1 < args.len() => {
                        page_len = args[i + 1].parse().unwrap_or(50);
                        i += 2;
                    }
                    _ => i += 1,
                }
            }
            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            let page = bitbucket::list_repos(&client, workspace, role, query, page_len)?;

            if output == "json" {
                println!("{}", serde_json::to_string_pretty(&page).unwrap());
                return Ok(());
            }

            println!(
                "{:<40} {:<12} {:<10} UPDATED",
                "FULL NAME", "LANGUAGE", "PRIVATE"
            );
            for r in &page.values {
                println!(
                    "{:<40} {:<12} {:<10} {}",
                    r.full_name,
                    r.language.as_deref().unwrap_or(""),
                    r.is_private,
                    r.updated_on.as_deref().unwrap_or("")
                );
            }
            Ok(())
        }
        "get" => {
            if args.len() < 3 {
                return Err("Usage: acli bb repo get <workspace> <slug>".to_string());
            }
            let workspace = &args[1];
            let slug = &args[2];
            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            let repo = bitbucket::get_repo(&client, workspace, slug)?;

            if output == "json" {
                println!("{}", serde_json::to_string_pretty(&repo).unwrap());
                return Ok(());
            }

            println!("Name:        {}", repo.full_name);
            println!("Description: {}", repo.description.as_deref().unwrap_or(""));
            println!("Language:    {}", repo.language.as_deref().unwrap_or(""));
            println!("SCM:         {}", repo.scm.as_deref().unwrap_or("git"));
            println!("Private:     {}", repo.is_private);
            println!(
                "Main Branch: {}",
                repo.main_branch
                    .as_ref()
                    .map(|b| b.name.as_str())
                    .unwrap_or("N/A")
            );
            println!("Created:     {}", repo.created_on.as_deref().unwrap_or(""));
            println!("Updated:     {}", repo.updated_on.as_deref().unwrap_or(""));
            Ok(())
        }
        "create" => {
            if args.len() < 3 {
                return Err(
                    "Usage: acli bb repo create <workspace> <slug> --name <name>".to_string(),
                );
            }
            let workspace = &args[1];
            let slug = &args[2];
            let mut name = None;
            let mut description = None;
            let mut language = None;
            let mut is_private = true;
            let mut i = 3;
            while i < args.len() {
                match args[i].as_str() {
                    "--name" if i + 1 < args.len() => {
                        name = Some(args[i + 1].as_str());
                        i += 2;
                    }
                    "--description" if i + 1 < args.len() => {
                        description = Some(args[i + 1].as_str());
                        i += 2;
                    }
                    "--language" if i + 1 < args.len() => {
                        language = Some(args[i + 1].as_str());
                        i += 2;
                    }
                    "--public" => {
                        is_private = false;
                        i += 1;
                    }
                    _ => i += 1,
                }
            }
            let name = name.ok_or("--name is required")?;
            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            let repo = bitbucket::create_repo(
                &client,
                workspace,
                slug,
                name,
                is_private,
                description,
                language,
            )?;

            if output == "json" {
                println!("{}", serde_json::to_string_pretty(&repo).unwrap());
            } else {
                println!("Created repository: {}", repo.full_name);
            }
            Ok(())
        }
        "delete" | "rm" => {
            if args.len() < 3 {
                return Err("Usage: acli bb repo delete <workspace> <slug>".to_string());
            }
            let workspace = &args[1];
            let slug = &args[2];
            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            bitbucket::delete_repo(&client, workspace, slug)?;
            println!("Deleted repository: {}/{}", workspace, slug);
            Ok(())
        }
        action => Err(format!("Unknown repo action: {}", action)),
    }
}

// ---------------------------------------------------------------------------
// bitbucket pr
// ---------------------------------------------------------------------------

fn handle_bb_pr(args: &[String], profile: Option<&str>, output: &str) -> Result<(), String> {
    if args.is_empty() {
        println!("PR Commands:");
        println!("  list <workspace> <slug> [--state OPEN]     List pull requests");
        println!("  get <workspace> <slug> <id>                Get pull request details");
        println!("  create <workspace> <slug>                  Create a pull request");
        println!("  approve <workspace> <slug> <id>            Approve a pull request");
        println!("  merge <workspace> <slug> <id>              Merge a pull request");
        println!("  decline <workspace> <slug> <id>            Decline a pull request");
        return Ok(());
    }

    match args[0].as_str() {
        "list" | "ls" => {
            if args.len() < 3 {
                return Err("Usage: acli bb pr list <workspace> <slug> [--state OPEN]".to_string());
            }
            let workspace = &args[1];
            let slug = &args[2];
            let mut state = None;
            let mut page_len = 50i32;
            let mut i = 3;
            while i < args.len() {
                match args[i].as_str() {
                    "--state" if i + 1 < args.len() => {
                        state = Some(args[i + 1].as_str());
                        i += 2;
                    }
                    "--limit" if i + 1 < args.len() => {
                        page_len = args[i + 1].parse().unwrap_or(50);
                        i += 2;
                    }
                    _ => i += 1,
                }
            }
            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            let page = bitbucket::list_prs(&client, workspace, slug, state, page_len)?;

            if output == "json" {
                println!("{}", serde_json::to_string_pretty(&page).unwrap());
                return Ok(());
            }

            println!(
                "{:<6} {:<10} {:<18} {:<22} SOURCE -> DEST",
                "ID", "STATE", "AUTHOR", "TITLE"
            );
            for pr in &page.values {
                let title: String = pr.title.chars().take(22).collect();
                println!(
                    "{:<6} {:<10} {:<18} {:<22} {} -> {}",
                    pr.id,
                    pr.state,
                    pr.author.display_name,
                    title,
                    pr.source.branch.name,
                    pr.destination.branch.name
                );
            }
            Ok(())
        }
        "get" => {
            if args.len() < 4 {
                return Err("Usage: acli bb pr get <workspace> <slug> <id>".to_string());
            }
            let workspace = &args[1];
            let slug = &args[2];
            let pr_id: i64 = args[3]
                .parse()
                .map_err(|_| "PR ID must be a number".to_string())?;
            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            let pr = bitbucket::get_pr(&client, workspace, slug, pr_id)?;

            if output == "json" {
                println!("{}", serde_json::to_string_pretty(&pr).unwrap());
                return Ok(());
            }

            println!("ID:          {}", pr.id);
            println!("Title:       {}", pr.title);
            println!("State:       {}", pr.state);
            println!("Author:      {}", pr.author.display_name);
            println!("Source:      {}", pr.source.branch.name);
            println!("Destination: {}", pr.destination.branch.name);
            println!("Created:     {}", pr.created_on.as_deref().unwrap_or(""));
            println!("Updated:     {}", pr.updated_on.as_deref().unwrap_or(""));
            if let Some(d) = &pr.description {
                if !d.is_empty() {
                    println!("Description:\n{}", d);
                }
            }
            Ok(())
        }
        "create" => {
            if args.len() < 3 {
                return Err("Usage: acli bb pr create <workspace> <slug> --title <t> --source <b> --dest <b>".to_string());
            }
            let workspace = &args[1];
            let slug = &args[2];
            let mut title = None;
            let mut source = None;
            let mut dest = None;
            let mut description = None;
            let mut i = 3;
            while i < args.len() {
                match args[i].as_str() {
                    "--title" if i + 1 < args.len() => {
                        title = Some(args[i + 1].as_str());
                        i += 2;
                    }
                    "--source" if i + 1 < args.len() => {
                        source = Some(args[i + 1].as_str());
                        i += 2;
                    }
                    "--dest" if i + 1 < args.len() => {
                        dest = Some(args[i + 1].as_str());
                        i += 2;
                    }
                    "--description" if i + 1 < args.len() => {
                        description = Some(args[i + 1].as_str());
                        i += 2;
                    }
                    _ => i += 1,
                }
            }
            let title = title.ok_or("--title is required")?;
            let source = source.ok_or("--source <branch> is required")?;
            let dest = dest.ok_or("--dest <branch> is required")?;
            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            let pr =
                bitbucket::create_pr(&client, workspace, slug, title, source, dest, description)?;

            if output == "json" {
                println!("{}", serde_json::to_string_pretty(&pr).unwrap());
            } else {
                println!("Created PR #{}: {}", pr.id, pr.title);
            }
            Ok(())
        }
        "approve" => {
            if args.len() < 4 {
                return Err("Usage: acli bb pr approve <workspace> <slug> <id>".to_string());
            }
            let workspace = &args[1];
            let slug = &args[2];
            let pr_id: i64 = args[3]
                .parse()
                .map_err(|_| "PR ID must be a number".to_string())?;
            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            bitbucket::approve_pr(&client, workspace, slug, pr_id)?;
            println!("PR #{} approved", pr_id);
            Ok(())
        }
        "merge" => {
            if args.len() < 4 {
                return Err(
                    "Usage: acli bb pr merge <workspace> <slug> <id> [--strategy <s>]".to_string(),
                );
            }
            let workspace = &args[1];
            let slug = &args[2];
            let pr_id: i64 = args[3]
                .parse()
                .map_err(|_| "PR ID must be a number".to_string())?;
            let mut strategy = None;
            let mut i = 4;
            while i < args.len() {
                if args[i] == "--strategy" && i + 1 < args.len() {
                    strategy = Some(args[i + 1].as_str());
                    i += 2;
                } else {
                    i += 1;
                }
            }
            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            let pr = bitbucket::merge_pr(&client, workspace, slug, pr_id, strategy)?;
            println!("PR #{} merged (state: {})", pr.id, pr.state);
            Ok(())
        }
        "decline" => {
            if args.len() < 4 {
                return Err("Usage: acli bb pr decline <workspace> <slug> <id>".to_string());
            }
            let workspace = &args[1];
            let slug = &args[2];
            let pr_id: i64 = args[3]
                .parse()
                .map_err(|_| "PR ID must be a number".to_string())?;
            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            let pr = bitbucket::decline_pr(&client, workspace, slug, pr_id)?;
            println!("PR #{} declined", pr.id);
            Ok(())
        }
        action => Err(format!("Unknown PR action: {}", action)),
    }
}

// ---------------------------------------------------------------------------
// bitbucket pipeline
// ---------------------------------------------------------------------------

fn handle_bb_pipeline(args: &[String], profile: Option<&str>, output: &str) -> Result<(), String> {
    if args.is_empty() {
        println!("Pipeline Commands:");
        println!("  list <workspace> <slug>                  List pipelines");
        println!("  get <workspace> <slug> <uuid>            Get pipeline details");
        println!("  run <workspace> <slug> --branch <b>      Trigger a pipeline");
        println!("  stop <workspace> <slug> <uuid>           Stop a pipeline");
        println!("  steps <workspace> <slug> <uuid>          List pipeline steps");
        println!("  log <workspace> <slug> <uuid> <step>     Get step log");
        return Ok(());
    }

    match args[0].as_str() {
        "list" | "ls" => {
            if args.len() < 3 {
                return Err("Usage: acli bb pipeline list <workspace> <slug>".to_string());
            }
            let workspace = &args[1];
            let slug = &args[2];
            let mut page_len = 25i32;
            let mut i = 3;
            while i < args.len() {
                if args[i] == "--limit" && i + 1 < args.len() {
                    page_len = args[i + 1].parse().unwrap_or(25);
                    i += 2;
                } else {
                    i += 1;
                }
            }
            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            let page = bitbucket::list_pipelines(&client, workspace, slug, page_len)?;

            if output == "json" {
                println!("{}", serde_json::to_string_pretty(&page).unwrap());
                return Ok(());
            }

            println!(
                "{:<8} {:<12} {:<12} {:<26} CREATED",
                "BUILD#", "STATUS", "TRIGGER", "TARGET"
            );
            for p in &page.values {
                let status = p
                    .state
                    .result
                    .as_ref()
                    .map(|r| r.name.as_str())
                    .unwrap_or(&p.state.name);
                let target = p.target.ref_name.as_deref().unwrap_or("");
                println!(
                    "{:<8} {:<12} {:<12} {:<26} {}",
                    p.build_number,
                    status,
                    p.trigger.name,
                    target,
                    p.created_on.as_deref().unwrap_or("")
                );
            }
            Ok(())
        }
        "get" => {
            if args.len() < 4 {
                return Err("Usage: acli bb pipeline get <workspace> <slug> <uuid>".to_string());
            }
            let workspace = &args[1];
            let slug = &args[2];
            let uuid = &args[3];
            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            let p = bitbucket::get_pipeline(&client, workspace, slug, uuid)?;

            if output == "json" {
                println!("{}", serde_json::to_string_pretty(&p).unwrap());
                return Ok(());
            }

            let status = p
                .state
                .result
                .as_ref()
                .map(|r| r.name.as_str())
                .unwrap_or(&p.state.name);
            println!("Build #:   {}", p.build_number);
            println!("UUID:      {}", p.uuid);
            println!("Status:    {}", status);
            println!("Trigger:   {}", p.trigger.name);
            println!("Target:    {}", p.target.ref_name.as_deref().unwrap_or(""));
            println!(
                "Commit:    {}",
                p.target
                    .commit
                    .as_ref()
                    .map(|c| c.hash.as_str())
                    .unwrap_or("")
            );
            println!("Created:   {}", p.created_on.as_deref().unwrap_or(""));
            println!("Completed: {}", p.completed_on.as_deref().unwrap_or(""));
            if let Some(secs) = p.duration_in_seconds {
                println!("Duration:  {}s", secs);
            }
            Ok(())
        }
        "run" => {
            if args.len() < 3 {
                return Err(
                    "Usage: acli bb pipeline run <workspace> <slug> --branch <b>".to_string(),
                );
            }
            let workspace = &args[1];
            let slug = &args[2];
            let mut branch = None;
            let mut i = 3;
            while i < args.len() {
                if args[i] == "--branch" && i + 1 < args.len() {
                    branch = Some(args[i + 1].as_str());
                    i += 2;
                } else {
                    i += 1;
                }
            }
            let branch = branch.ok_or("--branch <name> is required")?;
            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            let p = bitbucket::run_pipeline(&client, workspace, slug, branch)?;
            println!("Pipeline triggered: build #{} ({})", p.build_number, p.uuid);
            Ok(())
        }
        "stop" => {
            if args.len() < 4 {
                return Err("Usage: acli bb pipeline stop <workspace> <slug> <uuid>".to_string());
            }
            let workspace = &args[1];
            let slug = &args[2];
            let uuid = &args[3];
            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            bitbucket::stop_pipeline(&client, workspace, slug, uuid)?;
            println!("Pipeline {} stopped", uuid);
            Ok(())
        }
        "steps" => {
            if args.len() < 4 {
                return Err("Usage: acli bb pipeline steps <workspace> <slug> <uuid>".to_string());
            }
            let workspace = &args[1];
            let slug = &args[2];
            let uuid = &args[3];
            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            let steps = bitbucket::list_pipeline_steps(&client, workspace, slug, uuid)?;

            if output == "json" {
                println!("{}", serde_json::to_string_pretty(&steps).unwrap());
                return Ok(());
            }

            println!("{:<40} {:<12} {:<8} NAME", "UUID", "STATUS", "DURATION");
            for s in &steps.values {
                let status = s
                    .state
                    .result
                    .as_ref()
                    .map(|r| r.name.as_str())
                    .unwrap_or(&s.state.name);
                let dur = s
                    .duration_in_seconds
                    .map(|d| format!("{}s", d))
                    .unwrap_or_default();
                println!(
                    "{:<40} {:<12} {:<8} {}",
                    s.uuid,
                    status,
                    dur,
                    s.name.as_deref().unwrap_or("")
                );
            }
            Ok(())
        }
        "log" => {
            if args.len() < 5 {
                return Err(
                    "Usage: acli bb pipeline log <workspace> <slug> <pipeline-uuid> <step-uuid>"
                        .to_string(),
                );
            }
            let workspace = &args[1];
            let slug = &args[2];
            let pipeline_uuid = &args[3];
            let step_uuid = &args[4];
            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            let log =
                bitbucket::get_pipeline_log(&client, workspace, slug, pipeline_uuid, step_uuid)?;
            print!("{}", log);
            Ok(())
        }
        action => Err(format!("Unknown pipeline action: {}", action)),
    }
}

// ---------------------------------------------------------------------------
// alert
// ---------------------------------------------------------------------------

fn handle_alert(args: &[String], profile: Option<&str>, output: &str) -> Result<(), String> {
    if args.is_empty() {
        println!("Alert Commands:");
        println!("  list                  List active alerts");
        println!("  get <id>              Show alert details");
        println!("  create <message>      Create a new alert");
        println!("  ack <id>              Acknowledge an alert");
        println!("  close <id>            Close an alert");
        println!("  teams                 List all teams");
        println!("  schedules             List all schedules");
        println!("  oncall <schedule-id>  Show who is currently on-call");
        println!("  oncall --from <iso> --until <iso> [<schedule-id> ...]");
        println!("                        List who is on-call in a date/time range,");
        println!("                        across the given schedules (or all schedules)");
        return Ok(());
    }

    match args[0].as_str() {
        "list" | "ls" => {
            let mut status = None;
            let mut i = 1;
            while i < args.len() {
                if args[i] == "--status" && i + 1 < args.len() {
                    status = Some(args[i + 1].as_str());
                    i += 2;
                } else {
                    i += 1;
                }
            }
            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            let alert_list = alerts::list_alerts(&client, status)?;

            if output == "json" {
                println!("{}", serde_json::to_string_pretty(&alert_list).unwrap());
                return Ok(());
            }

            println!(
                "{:<40} {:<8} {:<8} {:<12} {:<8} MESSAGE",
                "ID", "TINYID", "STATUS", "ACKNOWLEDGED", "PRIORITY"
            );
            for alert in alert_list {
                let tiny_id = alert.tiny_id.unwrap_or_default();
                println!(
                    "{:<40} {:<8} {:<8} {:<12} {:<8} {}",
                    alert.id,
                    tiny_id,
                    alert.status,
                    alert.acknowledged,
                    alert.priority,
                    alert.message
                );
            }
            Ok(())
        }
        "get" => {
            if args.len() < 2 {
                return Err("Usage: acli alert get <id-or-alias>".to_string());
            }
            let identifier = &args[1];
            let mut explicit_type: Option<String> = None;
            let mut i = 2;
            while i < args.len() {
                if args[i] == "--type" && i + 1 < args.len() {
                    explicit_type = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    i += 1;
                }
            }
            let id_type = explicit_type
                .as_deref()
                .unwrap_or_else(|| alerts::infer_id_type(identifier));
            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            let alert = alerts::get_alert(&client, identifier, id_type)?;

            let notes = alerts::list_alert_notes(&client, &alert.id).unwrap_or_default();
            let logs = alerts::list_alert_logs(&client, &alert.id).unwrap_or_default();

            if output == "json" {
                let mut obj = serde_json::to_value(&alert).unwrap();
                obj["notes"] = serde_json::to_value(&notes).unwrap();
                obj["activityLog"] = serde_json::to_value(&logs).unwrap();
                println!("{}", serde_json::to_string_pretty(&obj).unwrap());
                return Ok(());
            }

            println!("ID:       {}", alert.id);
            println!("Tiny ID:  {}", alert.tiny_id.as_deref().unwrap_or("-"));
            println!("Alias:    {}", alert.alias.as_deref().unwrap_or("-"));
            println!("Message:  {}", alert.message);
            println!("Status:   {}", alert.status);
            println!("Acked:    {}", alert.acknowledged);
            println!("Priority: {}", alert.priority);
            println!("Created:  {}", alert.created_at);
            if let Some(desc) = &alert.description {
                if !desc.is_empty() {
                    println!("Desc:     {}", desc);
                }
            }
            if !alert.responders.is_empty() {
                println!("Responders:");
                for r in &alert.responders {
                    println!(
                        "  {} ({})",
                        r.id.as_deref().unwrap_or("-"),
                        r.responder_type.as_deref().unwrap_or("unknown")
                    );
                }
            }
            if notes.is_empty() {
                println!("Notes:    (none)");
            } else {
                println!("Notes:");
                for n in &notes {
                    let ts = n.created_at.as_deref().unwrap_or("-");
                    let owner = n.owner.as_deref().unwrap_or("unknown");
                    println!("  [{}] {}: {}", ts, owner, n.note);
                }
            }
            if !logs.is_empty() {
                println!("Activity:");
                for l in &logs {
                    let ts = l.log_time.as_deref().unwrap_or("-");
                    let owner = l.owner.as_deref().unwrap_or("system");
                    println!("  [{}] {}: {}", ts, owner, l.log);
                }
            }
            Ok(())
        }
        "create" => {
            if args.len() < 2 {
                return Err("Usage: acli alert create <message> [--description <d>] [--alias <a>] [--priority <p>]".to_string());
            }
            let message = args[1].clone();
            let mut description = None;
            let mut alias = None;
            let mut priority = None;
            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--description" if i + 1 < args.len() => {
                        description = Some(args[i + 1].clone());
                        i += 2;
                    }
                    "--alias" if i + 1 < args.len() => {
                        alias = Some(args[i + 1].clone());
                        i += 2;
                    }
                    "--priority" if i + 1 < args.len() => {
                        priority = Some(args[i + 1].clone());
                        i += 2;
                    }
                    _ => i += 1,
                }
            }
            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            let resp = alerts::create_alert(
                &client,
                alerts::CreateAlertPayload {
                    message,
                    description,
                    alias,
                    priority,
                },
            )?;
            println!("Alert created. Response: {}", resp);
            Ok(())
        }
        "ack" | "acknowledge" => {
            if args.len() < 2 {
                return Err(
                    "Usage: acli alert ack <id> [--note <n>] [--type <id|alias>]".to_string(),
                );
            }
            let identifier = &args[1];
            let mut id_type = "id";
            let mut note = None;
            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--type" if i + 1 < args.len() => {
                        id_type = &args[i + 1];
                        i += 2;
                    }
                    "--note" if i + 1 < args.len() => {
                        note = Some(args[i + 1].as_str());
                        i += 2;
                    }
                    _ => i += 1,
                }
            }
            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            let resp = alerts::acknowledge_alert(&client, identifier, id_type, note)?;
            println!("Alert acknowledged. Response: {}", resp);
            Ok(())
        }
        "close" => {
            if args.len() < 2 {
                return Err(
                    "Usage: acli alert close <id> [--note <n>] [--type <id|alias>]".to_string(),
                );
            }
            let identifier = &args[1];
            let mut id_type = "id";
            let mut note = None;
            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--type" if i + 1 < args.len() => {
                        id_type = &args[i + 1];
                        i += 2;
                    }
                    "--note" if i + 1 < args.len() => {
                        note = Some(args[i + 1].as_str());
                        i += 2;
                    }
                    _ => i += 1,
                }
            }
            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            let resp = alerts::close_alert(&client, identifier, id_type, note)?;
            println!("Alert closed. Response: {}", resp);
            Ok(())
        }
        "teams" => {
            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            let teams = alerts::list_teams(&client)?;

            if output == "json" {
                println!("{}", serde_json::to_string_pretty(&teams).unwrap());
                return Ok(());
            }

            println!("{:<40} {:<40} DESCRIPTION", "ID", "NAME");
            for team in teams {
                println!(
                    "{:<40} {:<40} {}",
                    team.id,
                    team.name,
                    team.description.unwrap_or_default()
                );
            }
            Ok(())
        }
        "schedules" => {
            let mut exclude_team: Option<String> = None;
            let mut i = 1;
            while i < args.len() {
                if args[i] == "--exclude-team" && i + 1 < args.len() {
                    exclude_team = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    i += 1;
                }
            }

            let cfg = Config::load()?;
            let prof = cfg.get_profile(profile)?;
            let escalation_schedules = prof
                .defaults
                .as_ref()
                .and_then(|d| d.escalation_schedules.clone());
            let client = Client::new(prof);
            let mut schedules = alerts::list_schedules(&client, escalation_schedules)?;

            if let Some(ref team_id) = exclude_team {
                schedules.retain(|s| s.team_id.as_ref() != Some(team_id));
            }

            if output == "json" {
                println!("{}", serde_json::to_string_pretty(&schedules).unwrap());
                return Ok(());
            }

            println!("{:<40} {:<30} {:<40} DESCRIPTION", "ID", "NAME", "TEAM_ID");
            for schedule in schedules {
                println!(
                    "{:<40} {:<30} {:<40} {}",
                    schedule.id,
                    schedule.name,
                    schedule
                        .team_id
                        .as_ref()
                        .unwrap_or(&"(no team)".to_string()),
                    schedule.description.unwrap_or_default()
                );
            }
            Ok(())
        }
        "oncall" => {
            let mut from: Option<String> = None;
            let mut until: Option<String> = None;
            let mut schedule_ids: Vec<String> = Vec::new();
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--from" if i + 1 < args.len() => {
                        from = Some(args[i + 1].clone());
                        i += 2;
                    }
                    "--until" if i + 1 < args.len() => {
                        until = Some(args[i + 1].clone());
                        i += 2;
                    }
                    other => {
                        schedule_ids.push(other.to_string());
                        i += 1;
                    }
                }
            }

            let cfg = Config::load()?;
            let prof = cfg.get_profile(profile)?;
            let client = Client::new(prof.clone());

            // Range mode: --from/--until given → list on-call assignments across
            // a date/time range, for the given schedules or all schedules.
            if let (Some(from), Some(until)) = (from, until) {
                let schedules = if schedule_ids.is_empty() {
                    let escalation_schedules = prof
                        .defaults
                        .as_ref()
                        .and_then(|d| d.escalation_schedules.clone());
                    alerts::list_schedules(&client, escalation_schedules)?
                } else {
                    let all = alerts::list_schedules(&client, None)?;
                    schedule_ids
                        .iter()
                        .filter_map(|id| {
                            all.iter().find(|s| &s.id == id || &s.name == id).cloned()
                        })
                        .collect()
                };

                if schedules.is_empty() {
                    println!("No matching schedules found.");
                    return Ok(());
                }

                let results =
                    alerts::get_oncall_timeline_for_schedules(&client, &schedules, &from, &until);

                if output == "json" {
                    println!("{}", serde_json::to_string_pretty(&results).unwrap());
                    return Ok(());
                }

                // Resolve each unique responder id to a display name once.
                let mut names: std::collections::HashMap<String, String> =
                    std::collections::HashMap::new();
                for sched in &results {
                    for period in &sched.periods {
                        if let Some(id) = period.responder.as_ref().and_then(|r| r.id.clone()) {
                            names.entry(id).or_default();
                        }
                    }
                }
                for (id, name) in names.iter_mut() {
                    let user = alerts::get_jira_user(&client, id).or_else(|_| alerts::get_user(&client, id));
                    *name = match user {
                        Ok(u) => u
                            .display_name
                            .or(u.full_name)
                            .or(u.username)
                            .or(u.email_address)
                            .unwrap_or_else(|| id.clone()),
                        Err(_) => id.clone(),
                    };
                }

                println!("{:<30} {:<25} {:<25} ON-CALL", "SCHEDULE", "START", "END");
                for sched in &results {
                    if let Some(err) = &sched.error {
                        println!("{:<30} (error: {})", sched.schedule_name, err);
                        continue;
                    }
                    if sched.periods.is_empty() {
                        println!("{:<30} (no on-call assignments in range)", sched.schedule_name);
                        continue;
                    }
                    for period in &sched.periods {
                        let who = period
                            .responder
                            .as_ref()
                            .and_then(|r| r.id.as_ref())
                            .and_then(|id| names.get(id).cloned())
                            .unwrap_or_else(|| "unassigned".to_string());
                        println!(
                            "{:<30} {:<25} {:<25} {}",
                            sched.schedule_name, period.start_date, period.end_date, who
                        );
                    }
                }
                return Ok(());
            }

            // Legacy mode: single schedule, current on-call only.
            if schedule_ids.len() != 1 {
                return Err(
                    "Usage: acli alert oncall <schedule-id>\n   or: acli alert oncall --from <iso> --until <iso> [<schedule-id> ...]"
                        .to_string(),
                );
            }
            let user_ids = alerts::get_oncall(&client, Some(schedule_ids[0].as_str()))?;

            if output == "json" {
                println!("{}", serde_json::to_string_pretty(&user_ids).unwrap());
                return Ok(());
            }

            if user_ids.is_empty() {
                println!("No on-call users found for this schedule.");
                return Ok(());
            }

            println!("On-call user IDs:");
            for user_id in user_ids {
                println!("  {}", user_id);
            }
            Ok(())
        }
        "user" => {
            if args.len() < 2 {
                return Err("Usage: acli alert user <user-id>".to_string());
            }
            let user_id = &args[1];
            let cfg = Config::load()?;
            let client = Client::new(cfg.get_profile(profile)?);
            let user = alerts::get_user(&client, user_id)?;

            if output == "json" {
                println!("{}", serde_json::to_string_pretty(&user).unwrap());
                return Ok(());
            }

            // Jira user fields
            if let Some(ref account_id) = user.account_id {
                println!("Account ID: {}", account_id);
            }
            if let Some(ref display_name) = user.display_name {
                println!("Name: {}", display_name);
            }
            if let Some(ref email) = user.email_address {
                println!("Email: {}", email);
            }
            if let Some(active) = user.active {
                println!("Active: {}", active);
            }
            if let Some(ref tz) = user.time_zone {
                println!("Timezone: {}", tz);
            }
            // Opsgenie user fields
            if let Some(ref id) = user.id {
                println!("User ID: {}", id);
            }
            if let Some(ref full_name) = user.full_name {
                println!("Full name: {}", full_name);
            }
            if let Some(ref username) = user.username {
                println!("Username: {}", username);
            }
            Ok(())
        }
        action => Err(format!("Unknown alert action: {}", action)),
    }
}
