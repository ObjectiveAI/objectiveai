//! What the stand-in's agents "say": scripted runs, one per image kind.
//!
//! Every piece is written as the JSON an agent container streams and read
//! back through the provider SDK's own `AgenticLoopChunk`, so a chunk this
//! file writes is one the real wire could carry. The tests run every
//! script through that parse; if Ronald reshapes a chunk, they fail.

use diverge_daemon_sdk::endpoints::agents::logs::server::response::Item;
use diverge_provider_sdk::endpoints::containers::agents::run::server::response::AgenticLoopChunk;
use serde_json::{Value, json};

use crate::catalog::Kind;

/// Tool-call ids unique across every run, as a real agent's would be.
static CALLS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

pub struct Step {
    pub delay_ms: u64,
    pub item: Item,
}

pub struct Script {
    pub steps: Vec<Step>,
    call: u32,
    chars_out: usize,
    chars_in: usize,
}

pub fn chunk(value: Value) -> Item {
    match serde_json::from_value::<AgenticLoopChunk>(value.clone()) {
        Ok(chunk) => Item::Chunk(chunk),
        Err(error) => panic!("the stand-in wrote a chunk the wire cannot carry: {error}\n{value}"),
    }
}

impl Script {
    pub fn new(input: &str) -> Self {
        Script { steps: Vec::new(), call: 0, chars_out: 0, chars_in: input.len() + 1600 }
    }

    fn push(&mut self, delay_ms: u64, value: Value) {
        self.steps.push(Step { delay_ms, item: chunk(value) });
    }

    fn with_parent(mut value: Value, parent: Option<&str>) -> Value {
        if let Some(parent) = parent {
            value["parent_tool_call_id"] = json!(parent);
        }
        value
    }

    /// Text arrives in pieces, a few words at a time.
    fn pieces(&mut self, kind: &str, parent: Option<&str>, text: &str, pace: u64) {
        self.chars_out += text.len();
        let words: Vec<&str> = text.split_inclusive(' ').collect();
        for (i, group) in words.chunks(3).enumerate() {
            let piece: String = group.concat();
            let delay = if i == 0 { pace * 4 } else { pace + (i as u64 * 7) % 40 };
            self.push(delay, Self::with_parent(json!({ "type": kind, "text": piece }), parent));
        }
    }

    pub fn think(&mut self, parent: Option<&str>, text: &str) {
        self.pieces("assistant_reasoning", parent, text, 35);
    }

    pub fn say(&mut self, parent: Option<&str>, text: &str) {
        self.pieces("assistant_text_content", parent, text, 55);
    }

    pub fn call(&mut self, parent: Option<&str>, name: &str, arguments: Value) -> String {
        self.call += 1;
        let id = format!("call_{}", CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
        let args = arguments.to_string();
        self.chars_out += args.len();
        self.push(500, Self::with_parent(json!({ "type": "assistant_tool_call", "id": id, "name": name, "arguments": args }), parent));
        id
    }

    pub fn answer(&mut self, parent: Option<&str>, id: &str, text: &str, is_error: bool, wait_ms: u64) {
        self.chars_in += text.len();
        self.push(
            wait_ms,
            Self::with_parent(
                json!({ "type": "tool_response", "id": id, "content": [{ "type": "text", "text": text }], "isError": is_error }),
                parent,
            ),
        );
    }

    pub fn notify(&mut self, message: Value, fatal: bool) {
        self.push(200, json!({ "type": "notification", "is_fatal": fatal, "message": message }));
    }

    /// Token counts derived from the script's own length — this is a
    /// stand-in, and the screen says so.
    pub fn usage(mut self) -> Vec<Step> {
        let prompt = (self.chars_in / 4) as u64;
        let completion = (self.chars_out / 4) as u64;
        self.push(
            150,
            json!({ "type": "usage", "prompt_tokens": prompt, "completion_tokens": completion, "total_tokens": prompt + completion }),
        );
        self.steps
    }
}

fn topic(input: &str) -> String {
    let line = input.lines().find(|l| !l.trim().is_empty()).unwrap_or("that").trim();
    let mut out: String = line.chars().take(72).collect();
    if line.chars().count() > 72 {
        out.push('…');
    }
    out
}

/// One run: what an agent of `kind` says in answer to `input`.
/// `read` reads a file off the stand-in host, so tool answers are real.
pub fn run(kind: Kind, input: &str, read: &dyn Fn(&str) -> String) -> Vec<Step> {
    let about = topic(input);
    let mut s = Script::new(input);
    match kind {
        Kind::Cc => {
            s.think(None, &format!("Before changing anything I want to see the project. The ask is: {about}"));
            let id = s.call(None, "Glob", json!({ "pattern": "/projects/site/**" }));
            s.answer(None, &id, "/projects/site/README.md\n/projects/site/index.html\n/projects/site/styles.css", false, 700);
            let id = s.call(None, "Read", json!({ "file_path": "/projects/site/index.html" }));
            s.answer(None, &id, &read("/projects/site/index.html"), false, 900);
            s.say(None, "It's one page with four links. Before I touch it I'll have a helper check every link, so we know what's actually broken.");
            let task = s.call(None, "Task", json!({ "description": "Check every link on index.html", "subagent_type": "general-purpose" }));
            let p = Some(task.as_str());
            s.think(p, "Four hrefs. Local paths first, then the mail link.");
            let id = s.call(p, "Bash", json!({ "command": "grep -o 'href=\"[^\"]*\"' /projects/site/index.html" }));
            s.answer(p, &id, "href=\"styles.css\"\nhref=\"/projects\"\nhref=\"/notes\"\nhref=\"mailto:hello@example.com\"\nhref=\"/old-page\"", false, 800);
            let id = s.call(p, "Bash", json!({ "command": "ls /projects/site/old-page" }));
            s.answer(p, &id, "ls: /projects/site/old-page: No such file or directory", true, 600);
            s.say(p, "One link goes nowhere: /old-page. The rest resolve.");
            s.answer(None, &task, "One broken link: /old-page (\"Archive\"). Everything else resolves.", false, 400);
            s.say(None, &format!("Here's where I landed on “{about}”:\n\n- The page is small and clean.\n- **One link is broken** — “Archive” points at /old-page, which doesn't exist.\n\nI haven't edited anything. Say the word and I'll remove the link or point it somewhere real."));
        }
        Kind::Codex => {
            s.think(None, "Search first, then decide.");
            let id = s.call(None, "shell", json!({ "command": ["rg", "-n", "TODO|FIXME", "/projects"] }));
            s.answer(None, &id, "(no matches)", false, 800);
            let id = s.call(None, "shell", json!({ "command": ["wc", "-l", "/projects/site/index.html", "/projects/site/styles.css"] }));
            let lines = read("/projects/site/index.html").lines().count();
            let css = read("/projects/site/styles.css").lines().count();
            s.answer(None, &id, &format!("  {lines} /projects/site/index.html\n   {css} /projects/site/styles.css\n  {} total", lines + css), false, 600);
            s.say(None, &format!("On “{about}”: there are no TODOs in the project, and it's two files. Nothing is blocking — tell me what to change and I'll write the patch."));
        }
        Kind::Hermes => {
            s.think(None, &format!("I should look at what's already written down before searching. Ask: {about}"));
            let id = s.call(None, "read_file", json!({ "path": "/notes/ideas.md" }));
            s.answer(None, &id, &read("/notes/ideas.md"), false, 700);
            let id = s.call(None, "web_search", json!({ "query": about }));
            s.answer(None, &id, "The stand-in daemon has no network, so this search returned nothing. A real run would list results here.", true, 1200);
            s.say(None, "I couldn't search, so here's what your own notes already say:\n\n1. A shop on your profile for the poster packs\n2. Borrowing the studio PC's GPU on weekends\n3. What a bounty looks like before money exists\n4. Onboarding without a tour\n\nThe third one is the open question the others hang on.");
        }
        Kind::Openrouter => {
            s.think(None, &format!("A plain question, no tools needed. Answer directly and keep it short. Ask: {about}"));
            s.say(None, &format!("On “{about}” — short answer: start with the smallest version that someone would actually use this week, and let what they do with it tell you what's missing.\n\nIf you want, give me the constraints and I'll turn that into three concrete options."));
        }
        Kind::Eliza => {
            s.notify(json!({ "message": "character loaded", "name": "Iris" }), false);
            s.say(None, &format!("Oh, I like this one. “{about}” — I'd start by asking who it's for, and then asking them. Want me to draft the three questions?"));
        }
        Kind::Python => {
            let code = "import pathlib\nfiles = sorted(p for p in pathlib.Path('/projects').rglob('*') if p.is_file())\nprint(len(files), 'files')\nfor f in files: print(f)";
            let id = s.call(None, "python", json!({ "code": code }));
            s.answer(None, &id, "3 files\n/projects/site/README.md\n/projects/site/index.html\n/projects/site/styles.css", false, 1100);
            s.say(None, &format!("Ran it. There are 3 files under /projects. Next step for “{about}”?"));
        }
    }
    s.usage()
}

/// The seeded agent that is mid-way through a long job: section after
/// section, a few seconds each, for about five minutes.
pub fn long_job(read: &dyn Fn(&str) -> String) -> Vec<Step> {
    let mut s = Script::new("Go through the whole site and tidy it, section by section.");
    s.think(None, "Long job. I'll go file by file and report as I go.");
    let files = ["/projects/site/index.html", "/projects/site/styles.css", "/projects/site/README.md"];
    for round in 1..=12 {
        let file = files[round % files.len()];
        let id = s.call(None, "Read", json!({ "file_path": file }));
        let body = read(file);
        s.answer(None, &id, &body, false, 2500);
        s.think(None, &format!("Pass {round}: {file} looks consistent with the last pass."));
        s.say(None, &format!("Pass {round} of 12 done ({file}). "));
        if let Some(last) = s.steps.last_mut() {
            last.delay_ms += 12000;
        }
    }
    s.say(None, "All twelve passes done. Nothing needed changing.");
    s.usage()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::ALL;

    #[test]
    fn every_script_is_carried_by_the_wire() {
        let read = |_: &str| "a file".to_string();
        for kind in ALL {
            let steps = run(kind, "Check the site for broken links", &read);
            assert!(steps.len() > 3, "{:?} said almost nothing", kind);
        }
        assert!(long_job(&read).len() > 20);
    }
}
