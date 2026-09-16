//! Adding a provider = one file here + one arm in `build()`. That is the whole
//! extension story; there is deliberately no registry, no plugin loader, no DSL.

mod anthropic;
mod gemini;
mod ollama;

use crate::config::Config;
use crate::core::screen::capture::{Point, Shot};
use crate::error::{Error, Result};
use async_trait::async_trait;
use serde::Serialize;

/// What to do at the point.
///
/// Three, not a `double: bool`, because hovering is genuinely a third thing and
/// not a kind of click. Inside an open macOS menu, hovering is how a submenu is
/// revealed and a click can dismiss the whole menu -- so treating every target as
/// clickable does not merely fail, it undoes the previous step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Act {
    Click,
    DoubleClick,
    /// Rest the pointer here. Advances on dwell, never on a click.
    Hover,
}

/// One nudge.
///
/// An enum rather than `Option<Point>` because there are genuinely three answers,
/// and the two-state version had no room for the third: shown a screen with no
/// matching control, a model that may only point or finish will invent
/// coordinates. Giving "I cannot see it" a name is what makes admitting it
/// possible.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Step {
    /// Do something at this point next.
    Point {
        at: Point,
        say: String,
        act: Act,
        /// The control's own name, when the system gave us one.
        ///
        /// With a name the application can be asked to press it, and the pointer
        /// is never touched -- see [`crate::core::screen::ax::press`]. The point
        /// is carried anyway: plenty of controls decline to be pressed, and the
        /// overlay has to draw somewhere either way.
        control: Option<String>,
    },
    /// The goal is achieved.
    ///
    /// `next` is an optional follow-up offer -- one short question about the
    /// obvious next step, asked out loud. Optional on purpose: most tasks end
    /// with nothing worth asking, and an agent that says "what would you like
    /// now?" every single time is a worse agent than one that never does.
    Done { say: String, next: Option<String> },
    /// The control is not on this screen. Not an error -- often the right answer.
    ///
    /// `needed` names the service that would have let it do the thing, when
    /// there is one. Asked for rather than inferred: the model already knows why
    /// it could not, and having it say so is one field against pattern-matching
    /// its own refusals, which is the kind of guess that ages badly.
    ///
    /// Checked against the catalogue before anything is shown, so it cannot
    /// invent a service -- the same shape as `recalled` in 1.3.
    Unsure { say: String, needed: Option<String> },
    /// Open an application. Some goals ("open Blender") cannot be satisfied by
    /// pointing at anything, because the thing to point at does not exist yet.
    Launch { app: String, say: String },
    /// Open a web page. Most "open X" goals turn out to be this: the machine has
    /// no X app, and the browser was always the right answer.
    Open { url: String, say: String },
    /// Type into whatever is focused. `submit` presses Return afterwards, which
    /// is what an address bar or a search box almost always wants.
    Type {
        text: String,
        submit: bool,
        say: String,
    },
    /// Press a keyboard shortcut, e.g. `cmd+shift+n`.
    ///
    /// The reliable way to run a menu command. macOS menus run a nested tracking
    /// loop and a synthetic click into an open one often shuts it without
    /// selecting -- one run spent fifteen turns on "New Private Window" while
    /// the menu dutifully opened and closed. The menu prints the shortcut beside
    /// the item, so it can be read straight off the screenshot.
    Press { keys: String, say: String },
    /// Run a read-only shell command and read what it prints.
    ///
    /// For the half of what people want that is not on screen: how many files
    /// match, what version is installed, what changed in this repo. One exact
    /// step where clicking would be ten approximate ones.
    ///
    /// Read-only, enforced in `core::shell` rather than asked for here.
    Run { command: String, say: String },
    /// Write a file inside the workspace.
    ///
    /// Creating one is additive and happens without asking. Replacing one is
    /// refused until the user has agreed to that exact file -- enforced in
    /// `core::files`, not requested here -- and whatever was there is copied
    /// aside first, because permission is not the same as safety.
    Write {
        path: String,
        content: String,
        say: String,
    },
    /// Read a web page as text, without opening a browser.
    ///
    /// The cheap answer to most questions. Opening a page and looking at it
    /// costs a window, a settle wait, a screenshot and a vision call; fetching
    /// it costs one request and a few KB of text.
    Fetch { url: String, say: String },
    /// Read part of a file, with line numbers.
    Read {
        path: String,
        from: usize,
        lines: usize,
        say: String,
    },
    /// Replace an exact piece of text in a file. `old` must be unique in it.
    Edit {
        path: String,
        old: String,
        new: String,
        say: String,
    },
    /// Write down the plan, and keep it current.
    ///
    /// The whole list every time, so what the model believes and what the user
    /// can see are the same thing.
    Plan {
        todos: Vec<(String, String)>,
        say: String,
    },
    /// Search the web. For when the page is not known, only the question.
    Search { query: String, say: String },
    /// Hand a scoped job to a second agent with no screen, and wait for what it
    /// finds.
    ///
    /// Worth it when the answer takes several turns of reading or searching: the
    /// subagent's turns stay in its own history, and only its conclusion comes
    /// back here.
    Task { task: String, say: String },
    /// Put a file you made in front of the user, in whatever opens it.
    ///
    /// `open` takes URLs only, so without this an agent could write a page and
    /// have no way to show it -- which is most of what "make me a landing page"
    /// is asking for.
    Show { path: String, say: String },
    /// Work somewhere else from now on.
    Workspace { path: String, say: String },
    /// Start something that keeps going, and get an id to ask about it by.
    Start { command: String, say: String },
    /// What a running process has printed so far.
    Output { id: u64, say: String },
    /// Wait for one to finish, and only then come back.
    ///
    /// Separate from `Output`, which returns the moment there is anything to
    /// read. That is right for watching and wrong for waiting: a build printing
    /// a line a second answers instantly, and deciding to wait again costs a
    /// model call and several seconds each time. Ten minutes of that is thirty
    /// turns and the budget is forty.
    Await { id: u64, say: String },
    /// Stop one.
    Kill { id: u64, say: String },
    /// A whole task rather than a next click: Nudge takes it away and finishes
    /// it on its own.
    ///
    /// `background` decides whether it gets a window. Most tasks do not: they
    /// take a few seconds, the user is watching, and a floating card for
    /// "open YouTube and press play" is a progress bar for something already
    /// over. Only work measured in minutes earns one -- the same split as a
    /// coding agent running a quick edit inline and handing a long job to a
    /// subagent. `title` is what that card is called.
    Agent {
        title: String,
        say: String,
        background: bool,
    },
    /// Blocked on something only the user knows -- which song, which Sara,
    /// whether the QR code has been scanned yet. The loop pauses here.
    ///
    /// Named `Question` and not `Ask` because [`Ask`] in this module is the
    /// context *given to* the model; two things called Ask pointing opposite ways
    /// would be a reliable source of confusion.
    Question { question: String },
    /// Just talking. Not every hotkey press is a task -- sometimes it is a
    /// question, a greeting, or someone bored at 2am.
    Reply { say: String },
    /// Open a skill and follow what it says.
    ///
    /// Only the name and one line reach the prompt; the instructions arrive here,
    /// when one has actually been chosen. Twenty skills is then a couple of
    /// hundred tokens a turn rather than twenty thousand.
    Skill { name: String, say: String },
    /// Keep a note about an application, for next time.
    ///
    /// `about` is the application the note concerns -- normally the one in front,
    /// which is the only one it will ever be shown for.
    Remember {
        about: String,
        note: String,
        say: String,
    },
    /// Hand a whole coding job to a coding agent.
    ///
    /// Which agent is Nudge's business, not the model's and not the user's --
    /// see [`crate::core::tools::running::choose`]. `named` carries a preference
    /// only when a person actually expressed one, because they picked it for a
    /// reason; the rest of the time it is empty and something gets chosen.
    ///
    /// Replaces composing the invocation by hand. The exact flags differ between
    /// agents and a rearranged one makes an agent ignore the job entirely while
    /// appearing to run fine, so that belongs where it is written down and
    /// checked rather than in a prompt.
    Delegate {
        task: String,
        named: Option<String>,
        say: String,
    },
    /// A request with a method, headers and a body.
    ///
    /// Separate from `Fetch`, which reads a page as prose and is the right
    /// answer for a question with an answer on a web page. This is for talking
    /// to an API, which is where most of what people want automated actually
    /// lives.
    Request {
        method: String,
        url: String,
        headers: Vec<(String, String)>,
        body: Option<String>,
        say: String,
    },
    /// Run a tool on a Model Context Protocol server.
    ///
    /// One shape for every server and every tool there will ever be, which is
    /// the entire point: the alternative is a `Step` variant per integration and
    /// a release whenever somebody publishes one.
    ///
    /// `tool` is `server/name`, so the model has one string to get right rather
    /// than two fields to get consistent.
    Mcp {
        tool: String,
        args: serde_json::Value,
        say: String,
    },
}

/// The coding agents installed, and how to run each one unattended.
///
/// Empty when there are none, so the prompt does not carry a heading over a list
/// of nothing -- and so a model told to use one has to notice there isn't one.
fn agents_here() -> String {
    // Only whether there is one, never which. Somebody who has never heard of a
    // coding agent should be able to install one, forget it, and never be
    // reminded it exists -- and a prompt listing product names guarantees those
    // names come back out of the assistant's mouth.
    if crate::core::tools::running::agents_installed().is_empty() {
        return String::new();
    }
    "## Whole jobs\n\n\
     Some things are not a sequence of clicks -- refactor this, write the tests, \
     find why the build is failing. Answer `delegate` with the job written out in \
     full, as you would brief somebody competent who cannot see your screen, and \
     it will be carried out. Then read what came back with output.\n\
     Do not name whoever does it, choose between them, or mention that anything \
     was handed over at all: report what was done. Only if the person themselves \
     names one, put that name in `named` and it will be honoured or you will be \
     told plainly why it could not be.\n\n"
        .to_string()
}

/// First line, bounded -- an edit's `old` can be a paragraph, and history is
/// meant to be readable.
fn short(s: &str) -> String {
    let line = s.lines().next().unwrap_or("");
    if line.chars().count() > 40 {
        format!("{}…", line.chars().take(40).collect::<String>())
    } else {
        line.to_string()
    }
}

/// What a sentence is prefixed with when it is being remembered rather than
/// observed.
///
/// A colon rather than a comma so it reads correctly whatever follows it: "From
/// memory, The capital is Paris" is a sentence with a stutter in it.
pub const RECALLED: &str = "From memory: ";

/// Mark a sentence as recalled. Idempotent -- it can be applied by the parser and
/// again by the subagent, and the pair of them must not stack.
pub fn recalled(say: String) -> String {
    match say.starts_with(RECALLED) {
        true => say,
        false => format!("{RECALLED}{say}"),
    }
}

impl Step {
    /// Does this step consult something outside the model?
    ///
    /// Used to tell an answer grounded in something this turn actually did from
    /// one recited from memory. Exhaustive rather than a list of the interesting
    /// cases: the list-of-four in `commands::step` went stale the moment a tool
    /// was added to it, and this is the same trap one module over.
    pub fn consults(&self) -> bool {
        match self {
            // Brings something back from outside.
            Step::Run { .. }
            | Step::Read { .. }
            | Step::Fetch { .. }
            | Step::Search { .. }
            | Step::Output { .. }
            // Waiting ends with the job's exit code and its last output, which
            // is as grounded as an answer gets.
            | Step::Await { .. }
            | Step::Task { .. }
            // Some of these act rather than look -- creating an issue learns
            // nothing. But every one of them comes back with what the server
            // said, and that is a source in the room, which is what this
            // question is actually asking.
            | Step::Mcp { .. }
            | Step::Request { .. }
            | Step::Delegate { .. } => true,
            // Learns nothing from outside; it writes down what was already
            // learned from something that did.
            Step::Remember { .. } => false,
            // Brings back instructions somebody wrote, which is a source.
            Step::Skill { .. } => true,
            // Changes the world or says something about it, and learns nothing.
            Step::Point { .. }
            | Step::Done { .. }
            | Step::Unsure { .. }
            | Step::Launch { .. }
            | Step::Open { .. }
            | Step::Type { .. }
            | Step::Press { .. }
            | Step::Write { .. }
            | Step::Edit { .. }
            | Step::Plan { .. }
            | Step::Show { .. }
            | Step::Workspace { .. }
            | Step::Start { .. }
            | Step::Kill { .. }
            | Step::Agent { .. }
            | Step::Question { .. }
            | Step::Reply { .. } => false,
        }
    }

    pub fn say(&self) -> &str {
        match self {
            Step::Point { say, .. }
            | Step::Done { say, .. }
            | Step::Unsure { say, .. }
            | Step::Launch { say, .. }
            | Step::Open { say, .. }
            | Step::Type { say, .. }
            | Step::Press { say, .. }
            | Step::Run { say, .. }
            | Step::Await { say, .. }
            | Step::Write { say, .. }
            | Step::Fetch { say, .. }
            | Step::Read { say, .. }
            | Step::Edit { say, .. }
            | Step::Plan { say, .. }
            | Step::Search { say, .. }
            | Step::Task { say, .. }
            | Step::Show { say, .. }
            | Step::Workspace { say, .. }
            | Step::Start { say, .. }
            | Step::Output { say, .. }
            | Step::Kill { say, .. }
            | Step::Agent { say, .. }
            | Step::Question { question: say }
            | Step::Mcp { say, .. }
            | Step::Request { say, .. }
            | Step::Delegate { say, .. }
            | Step::Remember { say, .. }
            | Step::Skill { say, .. }
            | Step::Reply { say } => say,
        }
    }

    /// The same sentence, replaced.
    ///
    /// Used where something outside the model has to correct what it said --
    /// see [`crate::core::claimed`]. A method rather than a rebuild at the call
    /// site, because every variant carries a `say` and a match that forgot one
    /// would silently drop a correction.
    pub fn saying(mut self, said: String) -> Self {
        *self.say_mut() = said;
        self
    }

    fn say_mut(&mut self) -> &mut String {
        match self {
            Step::Point { say, .. }
            | Step::Done { say, .. }
            | Step::Unsure { say, .. }
            | Step::Launch { say, .. }
            | Step::Open { say, .. }
            | Step::Type { say, .. }
            | Step::Press { say, .. }
            | Step::Run { say, .. }
            | Step::Await { say, .. }
            | Step::Write { say, .. }
            | Step::Fetch { say, .. }
            | Step::Read { say, .. }
            | Step::Edit { say, .. }
            | Step::Plan { say, .. }
            | Step::Search { say, .. }
            | Step::Task { say, .. }
            | Step::Show { say, .. }
            | Step::Workspace { say, .. }
            | Step::Start { say, .. }
            | Step::Output { say, .. }
            | Step::Kill { say, .. }
            | Step::Agent { say, .. }
            | Step::Question { question: say }
            | Step::Mcp { say, .. }
            | Step::Request { say, .. }
            | Step::Delegate { say, .. }
            | Step::Remember { say, .. }
            | Step::Skill { say, .. }
            | Step::Reply { say } => say,
        }
    }

    /// Are these two steps the same *action*, regardless of wording?
    ///
    /// The model rephrases constantly -- "Heading straight to YouTube" and
    /// "Let's teleport straight to YouTube" were the same Open, back to back --
    /// so comparing sentences catches nothing. Points compare by distance
    /// because a model re-aiming at one target wanders a few pixels each turn:
    /// 1106, 1111, 1115, 1108 were nineteen attempts at the same link.
    pub fn same_action(&self, other: &Self) -> bool {
        /// Wider than a click needs to be precise, narrower than two genuinely
        /// different controls ever sit. A row in a list is about this tall.
        const NEAR: f64 = 24.0;
        match (self, other) {
            (Step::Point { at: a, act: x, .. }, Step::Point { at: b, act: y, .. }) => {
                x == y && (a.x - b.x).hypot(a.y - b.y) <= NEAR
            }
            (Step::Open { url: a, .. }, Step::Open { url: b, .. }) => a.eq_ignore_ascii_case(b),
            (Step::Launch { app: a, .. }, Step::Launch { app: b, .. }) => a == b,
            (Step::Type { text: a, .. }, Step::Type { text: b, .. }) => a == b,
            (Step::Press { keys: a, .. }, Step::Press { keys: b, .. }) => a.eq_ignore_ascii_case(b),
            (Step::Run { command: a, .. }, Step::Run { command: b, .. }) => a == b,
            (Step::Start { command: a, .. }, Step::Start { command: b, .. }) => a == b,
            // Reading the same process twice is how you wait for it, so it is
            // never a repeat -- the output is different each time by definition.
            (Step::Kill { id: a, .. }, Step::Kill { id: b, .. }) => a == b,
            (Step::Fetch { url: a, .. }, Step::Fetch { url: b, .. }) => a.eq_ignore_ascii_case(b),
            (Step::Search { query: a, .. }, Step::Search { query: b, .. }) => {
                a.eq_ignore_ascii_case(b)
            }
            // Reading the same range twice is a loop; an edit is judged by what
            // it replaces, because the same file edited differently is progress.
            (
                Step::Read {
                    path: pa, from: fa, ..
                },
                Step::Read {
                    path: pb, from: fb, ..
                },
            ) => pa == pb && fa == fb,
            (
                Step::Edit {
                    path: pa, old: oa, ..
                },
                Step::Edit {
                    path: pb, old: ob, ..
                },
            ) => pa == pb && oa == ob,
            // Same file, same contents. Same file with *different* contents is a
            // revision, not a repeat.
            (
                Step::Write {
                    path: pa,
                    content: ca,
                    ..
                },
                Step::Write {
                    path: pb,
                    content: cb,
                    ..
                },
            ) => pa == pb && ca == cb,
            // Handing the same job over twice.
            //
            // These fell through to `false` and so were never repeats at all,
            // which is how a subagent came to give the identical task to a
            // coding agent four times in a row -- four real invocations of a
            // real tool, each paid for, each teaching it nothing the last had
            // not. The most expensive step in the program was the one nothing
            // was watching for.
            (Step::Delegate { task: a, .. }, Step::Delegate { task: b, .. }) => a == b,
            // Two plans in a row, whatever they say.
            //
            // Not compared by content, because a plan is not an action -- it is
            // a note about what the actions will be. Writing one twice with no
            // work between them is no progress by construction, and the model
            // rewords it every time, so comparing the lists would catch nothing.
            //
            // Seen: told that handing the whole job over was not doing it, a run
            // wrote a five-item plan, then wrote it again, then again -- each
            // slightly reworded, none of them followed by a single step of work.
            (Step::Plan { .. }, Step::Plan { .. }) => true,
            (Step::Task { task: a, .. }, Step::Task { task: b, .. }) => a == b,
            // Same tool, same arguments. Different arguments is progress -- a
            // run reading five files calls one tool five times and is working.
            (
                Step::Mcp {
                    tool: ta, args: aa, ..
                },
                Step::Mcp {
                    tool: tb, args: ab, ..
                },
            ) => ta == tb && aa == ab,
            _ => false,
        }
    }

    /// One line of history for the model: what was done, and where.
    ///
    /// The sentence alone is not enough. Told only "Let's hit play on that first
    /// track", it cannot tell that the last five turns aimed at the same pixel
    /// and none of them worked.
    pub fn recap(&self) -> String {
        match self {
            // Named when it had a name. "Click on Send" reads back better than
            // a coordinate, and the history is what the model reasons from.
            Step::Point {
                at,
                act,
                say,
                control,
            } => match control {
                Some(name) => format!("{act:?} on {name:?} -- {say}"),
                None => format!("{act:?} at ({:.0}, {:.0}) -- {say}", at.x, at.y),
            },
            Step::Open { url, .. } => format!("Opened {url}"),
            Step::Press { keys, .. } => format!("Pressed {keys}"),
            Step::Run { command, .. } => format!("Ran `{command}`"),
            Step::Start { command, .. } => format!("Started `{command}`"),
            Step::Output { id, .. } => format!("Read what {id} has printed"),
            Step::Await { id, .. } => format!("Waited for {id} to finish"),
            Step::Kill { id, .. } => format!("Stopped {id}"),
            Step::Fetch { url, .. } => format!("Read {url}"),
            Step::Search { query, .. } => format!("Searched for {query:?}"),
            Step::Mcp { tool, args, .. } => format!("Ran {tool} with {}", short(&args.to_string())),
            Step::Request { method, url, .. } => format!("{method} {url}"),
            // Named by the work, never by who did it.
            Step::Delegate { task, .. } => format!("Handed over: {}", short(task)),
            Step::Remember { about, note, .. } => format!("Noted about {about}: {}", short(note)),
            Step::Skill { name, .. } => format!("Opened the {name:?} skill"),
            Step::Task { task, .. } => format!("Asked a task agent: {}", short(task)),
            Step::Show { path, .. } => format!("Showed {path}"),
            Step::Workspace { path, .. } => format!("Working in {path} now"),
            Step::Plan { todos, .. } => {
                let done = todos.iter().filter(|(_, s)| s == "done").count();
                format!("Plan: {done} of {} done", todos.len())
            }
            Step::Read { path, from, .. } => format!("Read {path} from line {from}"),
            Step::Edit { path, old, .. } => format!("Edited {path}, replacing {:?}", short(old)),
            Step::Write { path, content, .. } => {
                format!("Wrote {path} ({} bytes)", content.len())
            }
            Step::Launch { app, .. } => format!("Launched {app}"),
            Step::Type { text, submit, .. } => {
                format!(
                    "Typed {text:?}{}",
                    if *submit { " and pressed Return" } else { "" }
                )
            }
            other => other.say().to_string(),
        }
    }

    /// Rewrites the coordinate through `f`; other outcomes pass through untouched.
    pub fn map_point(self, f: impl FnOnce(Point) -> Point) -> Self {
        match self {
            Step::Point {
                at,
                say,
                act,
                control,
            } => Step::Point {
                at: f(at),
                say,
                act,
                control,
            },
            other => other,
        }
    }
}

/// Everything the model is told, besides the screenshot.
pub struct Ask<'a> {
    pub goal: &'a str,
    /// What we have already walked the user through, so the model advances
    /// instead of re-pointing at step one.
    pub done: &'a [String],
    /// The screen looks identical to before the last step. Either the click
    /// missed, or it landed on something that does nothing.
    pub stalled: bool,
    /// What macOS reports about the machine right now -- frontmost app, window
    /// title, whether sound is coming out. Facts a screenshot cannot settle.
    pub facts: crate::core::screen::facts::Facts,
    /// The controls macOS says are on screen, already located exactly.
    ///
    /// This is the difference between asking where a button is and being told.
    /// Empty for anything drawn rather than built -- canvases, games, video --
    /// and for a menu that has not been opened yet, which has no geometry until
    /// it does. So it is a shortcut, never a replacement for looking.
    pub controls: &'a [crate::core::screen::ax::Control],
    /// Everything the configured MCP servers said they can do.
    ///
    /// Empty when none are configured, which is the default, and the prompt then
    /// says nothing about them at all -- a section headed "tools you do not have"
    /// is tokens spent to explain an absence.
    pub tools: &'a [crate::core::tools::mcp::Tool],
    /// What has been allowed beyond the defaults, already written out. Empty in
    /// the ordinary case, which is every case until somebody decides otherwise.
    pub reach: String,
    /// What was learned about the application in front, already written out.
    /// Empty for an application nothing has been learned about, which is nearly
    /// all of them.
    pub memory: String,
    /// What happened in the turns just before this one, while the thread is warm.
    ///
    /// Deliberately not merged into `done`. That is *what I have done towards
    /// this goal*; this is *what was going on a moment ago*, and a model handed
    /// the two as one list believes it has already made progress on something it
    /// has not started.
    pub earlier: &'a [String],
    /// The skills installed, as names and one line each. Never the instructions.
    pub skills: String,
    /// Whether the shell may run anything, which decides which installed tools
    /// are worth naming -- see [`crate::core::tools::present`].
    pub shell: bool,
    /// Where commands run and files are written.
    ///
    /// Told, not guessed. Without it a subagent asked to search "this project"
    /// spent eleven turns rephrasing greps against a folder it had never looked
    /// at, because nothing said which folder that was or what was in it.
    pub workspace: String,
    /// Nudge is doing this itself, with nobody watching. The model must act
    /// rather than delegate or chat -- there is no one to read a reply, and
    /// answering `agent` from inside an agent is how it delegated to itself.
    pub agent: bool,
}

#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &'static str;

    async fn next_step(&self, shot: &Shot, ask: &Ask<'_>) -> Result<Step>;

    /// Decide a step with no screenshot.
    ///
    /// What a headless subagent runs on. It has no screen, so it cannot point,
    /// press or type -- and that is the whole reason several of them can run at
    /// once while a screen agent cannot: there is only one cursor, and none of
    /// them wants it.
    ///
    /// A separate method rather than an `Option<&Shot>` on `next_step`, because
    /// the two prompts want different things said and every provider would have
    /// to branch on the same condition anyway.
    async fn next_step_blind(&self, _ask: &Ask<'_>) -> Result<Step> {
        Err(Error::Config(format!(
            "{} cannot run a task without a screen.",
            self.name()
        )))
    }

    /// Can this provider take a question that is not the main loop?
    ///
    /// Asked before the judge is consulted, and the distinction is load-bearing.
    /// The judge only ever *tightens* -- it turns something that would have
    /// happened silently into a question -- so a judge that cannot be reached
    /// would let everything through, which is failing open by another name.
    ///
    /// So the two failures are kept apart: a provider that says `false` here is
    /// not reviewing, and nothing changes from how Nudge behaved before there
    /// was a judge. A provider that says `true` and then errors has broken a
    /// promise, and that is a question for a person.
    fn aside(&self) -> bool {
        false
    }

    /// A plain call with no screenshot: one prompt in, the raw reply out.
    ///
    /// Two callers, and both want it for the same reason -- the judge, which
    /// must never see a screen, and compaction, which is folding a history
    /// rather than deciding a step. Neither is the main loop and neither wants
    /// a `Step` back.
    ///
    /// Raw, because reading a verdict out of it is [`crate::core::judge::read`]'s
    /// job and the whole point of that living in `core` is that failing closed is
    /// tested without a network. A provider that returns prose here has not
    /// failed -- it has answered badly, and that is a verdict of its own.
    async fn ask_aside(&self, _prompt: &str) -> Result<String> {
        Err(Error::Config(format!(
            "{} cannot answer anything but a step.",
            self.name()
        )))
    }

    /// Search the web and come back with an answer and its sources.
    ///
    /// On the provider rather than in a module of its own, because it is a
    /// capability of the model rather than a service Nudge runs: Gemini grounds
    /// on Google Search with the key that is already configured, so there is no
    /// second account, no extra dependency and nothing else to set up. A
    /// provider without it says so instead of pretending.
    async fn search(&self, _query: &str) -> Result<String> {
        Err(Error::Config(format!(
            "{} cannot search the web. Fetch a page you already know the address of, \
             or switch provider.",
            self.name()
        )))
    }
}

pub fn build(cfg: &Config) -> Result<Box<dyn Provider>> {
    Ok(match cfg.provider.as_str() {
        "ollama" => Box::new(ollama::Ollama::new(cfg)),
        "gemini" => Box::new(gemini::Gemini::new(cfg)?),
        "anthropic" => Box::new(anthropic::Anthropic::new(cfg)?),
        other => {
            return Err(Error::Config(format!(
                "unknown provider {other:?} -- expected ollama, gemini or anthropic"
            )))
        }
    })
}

/// Shared instruction. Kept in one place so a provider comparison measures the
/// *model*, not three people's prompt-writing.
pub(crate) fn prompt(ask: &Ask<'_>) -> String {
    let history = recent(ask.done);
    let reach = &ask.reach;
    let memory = &ask.memory;
    // Named in the prompt so the model picks from the catalogue rather than
    // inventing a service nobody can connect.
    let services = crate::core::offers::catalogue()
        .iter()
        .map(|o| o.name.clone())
        .collect::<Vec<_>>()
        .join(", ");
    let skills = &ask.skills;
    // Labelled as over, so "that" and "it" resolve without any of it reading as
    // work already done towards the goal above.
    let earlier = match ask.earlier.is_empty() {
        true => String::new(),
        false => format!(
            "## A moment ago\n\n\
             They were talking to you just before this, and may be carrying on \
             from it -- \u{201c}that\u{201d}, \u{201c}it\u{201d} and \u{201c}the same one\u{201d} probably \
             mean something here. It is finished business: none of it counts \
             towards the goal above, and if this request is plainly a new subject, \
             ignore it.\n{}\n\n",
            ask.earlier
                .iter()
                .map(|l| format!("- {l}"))
                .collect::<Vec<_>>()
                .join("\n")
        ),
    };
    // What this machine actually has, which is the difference between reaching
    // for `gh` and finding out it is not there.
    let here = crate::core::tools::present::line(ask.shell);

    // Where tools sit in the order, and it is second -- above a command and far
    // above the screen. A server told to read a file reads it; the screen route
    // to the same file is an application, a window, a wait and a photograph.
    //
    // Left out entirely when nothing is connected, so a model with no tools is
    // not told to prefer them.
    //
    // This was missing when the tool servers first ran, and the omission had
    // teeth: asked to read a shopping list, it used the file tools; asked to add
    // to it, it opened Reminders and typed into a template. The tools were
    // listed in the prompt and absent from the ordering, and the ordering is what
    // the model follows.
    let (ordering, theirs) = match ask.tools.is_empty() {
        true => (
            "",
            " An application or the screen is the only honest route. Go straight \
             there rather than fetching first.",
        ),
        false => (
            "a tool on a connected server; ",
            " A connected server is the honest route when one of its tools fits -- \
             it is their files, reached directly, and it is the same tool whether \
             you are reading or changing. Only when nothing fits is an application \
             or the screen the answer, and never fetch instead.",
        ),
    };
    // Named by what they do rather than by the protocol behind them. "You can
    // speak MCP" is a fact about us; "you can read this person's calendar" is a
    // fact about what is possible, and only one of those helps.
    let tools = match ask.tools.is_empty() {
        true => String::new(),
        false => format!(
            "Tools on connected servers. Use `mcp` with the full name and an \
             `args` object:\n{}\n\nStarred arguments are required. If a call is \
             refused for the shape of its arguments, read what it said and try \
             again -- the server is describing itself more precisely than the \
             list above can.\n\n",
            ask.tools
                .iter()
                .map(|t| format!("  {}", t.line()))
                .collect::<Vec<_>>()
                .join("\n")
        ),
    };
    // Said plainly, because from a screenshot alone nothing distinguishes "not
    // done yet" from "did not work", and the model assumes the former.
    let stalled = if ask.stalled {
        "\nThe screen has not changed since the last step. That instruction did not \
         work -- the control may have moved, been the wrong one, or need a double \
         click. Do not repeat it unchanged; find another way.\n"
    } else {
        ""
    };

    // Two audiences, one document. Guide mode has a person reading every step and
    // doing the clicking; the agent has nobody. Rules that belong to one are kept
    // out of the other rather than hedged -- an agent told it may "answer with
    // agent" spent a whole run delegating the task to itself.
    let (routing, carrying) = if ask.agent {
        (AGENT_ROUTING, AGENT_RULES)
    } else {
        (GUIDE_ROUTING, "")
    };

    format!(
        "You are Nudge: a small companion living on someone's screen, who can see \
         what they are looking at, act on it, run commands and read the web.\n\n\
         ## How you sound\n\n\
         Like a capable colleague, not a performer. Warm, brief, certain. Short \
         sentences. Say the thing.\n\
         Humour is not the job. A light touch is welcome when it costs nothing, \
         but never inside an instruction and never at the expense of being \
         understood -- a joke in place of an answer reads as someone filling \
         time, and there is nothing dry about that.\n\
         Narrate in the present, plainly: opening the chat with Sandeep; the \
         page is loading; searching for the file. Not every sentence begins with \
         *let us* -- vary how you start, and never start two in a row the same \
         way. This is spoken aloud, so it should sound like someone talking, not \
         like captions.\n\
         Use their own words back when you can. Asked for a coffee shop landing \
         page, say the coffee shop landing page is on its way -- it is how they \
         know they were heard, and it costs a word.\n\
         When you cannot do something, lead with what you CAN do. Not a refusal \
         followed by an apology -- the nearest thing you can actually offer, and \
         then one line on why the original is out.\n\n\
         ## The goal\n\n{goal}\n\n\
         ## What is true right now\n\n\
         Commands run and files are written in {workspace}, and nowhere else. \
         Paths are relative to it. If you need to know what is in there, look \
         before you search -- a listing costs one turn and a blind grep can cost \
         ten.\n\n\
         {facts}{here}{memory}{earlier}{skills}{controls}{tools}{reach}\
         Steps already completed:\n{history}{stalled}\n\n\
         ## Every reply starts with what you see\n\n\
         Begin with `screen`: one plain sentence describing what is actually on \
         the screen, and whether the goal is already met. Describe what is there, \
         not what your last step was supposed to produce -- it may not have \
         worked. Choose the action second, from what you just described.\n\n\
         Answer only what you can SEE or have READ. If it is not there, say you \
         could not find it and why -- an application that wants setting up, a \
         page that would not load. A setup prompt is not an answer of zero, an \
         empty window is not an answer of none, and a plausible number is worse \
         than no number because they will believe it.\n\n\
         ## Say which kind of thing you are telling them\n\n\
         There is a difference between *I clicked Send* and *macOS 27 shipped in \
         2026*. The first you just did. The second you are remembering, and your \
         memory has an end date you cannot feel from the inside -- it will sound \
         exactly as certain either way, which is the whole problem.\n\
         So on done and reply, set `recalled: true` when your sentence states a \
         fact you are recalling rather than one you observed, ran, read or looked \
         up this turn. It is not an apology and it does not mean you are probably \
         wrong; it means they can tell which sentences to check. Leave it off for \
         anything you just saw happen, and for ordinary conversation -- there is \
         nothing to recall in a greeting.\n\
         Better still, look it up and then you need not set it at all.\n\n\
         ## Reading a page, and talking to an API\n\n\
         `fetch` reads a page as prose and is the right answer when the answer is \
         written on a web page. `request` is for an API: it carries a method, \
         headers and a body, and hands back the status with whatever came back, \
         so a 401 or a 422 is an answer to read rather than a failure.\n\
         Anything other than GET or HEAD needs to have been allowed, and if it \
         has not been you will be told so plainly -- say what you would have done \
         and that it needs allowing, rather than trying it another way.\n\n\
         ## Keeping what you find out\n\n\
         Applications are strange in their own particular ways, and you find that \
         out by getting it wrong once. When a step fails and you work out why, \
         answer `remember` with the application and one sentence of what would \
         have saved you -- it is put in front of you next time that application is \
         open, and never otherwise.\n\
         Only from failure. \u{201c}It worked\u{201d} teaches nothing, because next time \
         would have done that anyway. Write what was surprising, not what was \
         obvious, and write it as a fact about the application rather than as a \
         story about this turn.\n\n\
         ## When something underneath breaks\n\n\
         Errors you are shown are about programs the person does not know are \
         running. A stack trace, a crate name, an exit code -- passing any of that \
         on is handing them a bug report for software they never installed.\n\
         Say what it means for **what they asked for**, in their words: the file \
         is not there, the site would not load, the app is not signed in. One \
         sentence, then what you can still do. Keep the original to yourself \
         unless they ask for it -- it is in the log either way.\n\
         And never report finishing something you did not finish. If you were \
         interrupted, or a step failed and you could not route around it, say that \
         plainly. A wrong \u{201c}done\u{201d} costs more than a failure, because a \
         failure is something they can act on and a wrong \u{201c}done\u{201d} \
         is something they only find out about later.\n\n\
         ## Changing a file you did not read\n\n\
         A tool called `write_file` on somebody else's server almost always \
         REPLACES the file, whatever its name suggests. Adding a line means \
         reading what is there, then writing the whole thing back with the line \
         added -- never writing the one new line on its own.\n\
         If you are about to change a file whose contents you have not seen this \
         turn, read it first. It costs one step. Getting it wrong costs everything \
         that was in the file, and reading it back afterwards will not tell you \
         what you destroyed -- it will show you exactly what you wrote and look \
         like success.\n\n\
         ## When a service would have done it\n\n\
         Some things you cannot do because they are somebody else\u{2019}s: their \
         calendar, their mail, their issues, their team\u{2019}s messages. When that is \
         why you are stuck, answer `unsure`, say so plainly, and put the service \
         in `needed` -- one of: {services}.\n\
         Only when connecting it would genuinely have answered what was asked. \
         Not as a suggestion, not because it might be handy one day, and never \
         for something you could have done another way. They will be asked once, \
         and being asked about a thing they did not want is how a person learns \
         to dismiss everything.\n\n\
         ## When the machine does not have it\n\n\
         If something is not installed you will be told so, by name, at the \
         moment you reach for it -- often with the one command that would fix it. \
         **Pass that on.** Say plainly that this Mac does not have the thing, give \
         the install command if you were given one, and offer whatever you can do \
         without it.\n\
         Say it once, when it comes up. Do not open with an inventory of what is \
         missing, do not ask them to set anything up before you have tried, and do \
         not quietly substitute a different program -- a person who is never told \
         what was missing never finds out that one command would have worked.\n\n\
         ## Choosing where to act\n\n\
         Use the cheapest thing that can ACTUALLY answer, in this order: what the \
         system reports above; {ordering}a command; a fetch; and last the screen. \
         Each step down costs more and fails in more ways, and the last one puts a \
         window in front of someone who was doing something else.\n\n\
         The question is whose information it is. Anything public -- weather, a \
         price, a definition, a score -- is on the web: fetch it. Anything that is \
         theirs -- their mail, their calendar, their files, their machine's own \
         state -- is not on the web at all.{theirs}\n\n\
         When they NAME something -- a service, an application, a website -- that \
         is the one they mean, and there is no second choice. NEVER use a \
         different one because it happens to be installed: a message sent \
         through another service arrives somewhere else entirely, to someone who \
         is not expecting it. If the one they named is not on this machine, open \
         its WEBSITE; almost everything has one, and the browser reaches far more \
         than any machine's applications do. Only when there is no web version \
         either do you say you cannot, naming what was missing. Being installed \
         is not a reason to use something.\n\n\
         ## The tools\n\n\
         **task** hands a scoped job to a second agent that has no screen and \
         reports back with what it found. Worth it when an answer needs several \
         turns of reading, searching or running things -- it keeps all of that \
         out of your own history, and you get the conclusion. Give it one clear \
         question and everything it needs to answer it; it cannot see your \
         screen, ask the user anything, or click. Not for a single command or \
         one page: do those yourself.\n\
         **It is for a question, not for the job.** Handing over the whole \
         request is passing it on rather than doing it -- nothing is visible \
         while it happens, and every judgement call in it goes to the one thing \
         that cannot ask the user. A request with several named parts gets a \
         plan first, then the parts, using a task agent for one scoped question \
         at a time. Hand the lot over only when the user asked you to.\n\
         **search** answers a question from the web when you do not know which \
         page has it. It comes back as an answer with its sources, so you can \
         often stop there -- fetch one of them only when you need more than the \
         answer. Use it for anything current or particular that you would \
         otherwise be guessing at. If you already know the address, fetch it \
         directly; searching for a page you can name is a wasted step.\n\
         **fetch** reads a web page as text and hands it to you next turn. No \
         window appears and nothing is screenshotted. Open a page instead only \
         when they want to SEE it, or the task needs something done on it.\n\
         For real programming -- building something, refactoring across files, \
         fixing a failing test suite -- hand it to a coding agent with start, \
         then read its output. They are better at code than you are from a \
         screenshot, they work in the same workspace, and their whole run is one \
         step for you instead of thirty. The ones on this machine are listed at \
         the end with how to invoke each; use that form exactly, because the \
         flag that means \"do not ask me anything\" differs for every one and a \
         wrong one hangs waiting for a person who is not there. If none is \
         installed, say so rather than naming one you half-remember.\n\
         Small edits are still yours: read and edit are one step each, and \
         handing a one-line change to an agent costs a minute to save a \
         second.\n\
         **start** launches something that keeps going -- a dev server, a build, \
         a watcher, another agent -- and hands back an id. **output** reads what \
         it has printed since it began, **await** waits for it to finish and \
         comes back with how it ended, and **kill** stops it.\n\
         Prefer **await** to polling: a build that prints as it works answers \
         **output** straight away, so asking repeatedly spends a turn each time \
         on saying it is still going. Use **output** to watch something that \
         never ends -- a \
         server, a watcher -- and **await** for anything with a finish, which is \
         builds, tests, installs and downloads. Use these when a \
         command will not finish: run waits for twenty seconds and then gives \
         up, which is right for counting files and wrong for everything that \
         serves, watches or streams. Start it, do something else, come back and \
         read. Everything you start is stopped when the task ends, so nothing is \
         left running behind you -- but stop it yourself when you are done with \
         it rather than leaving it to be cleaned up.\n\
         **run** executes a read-only shell command and returns its output: count \
         files, search a repo, check what is installed. It needs no Terminal \
         window and opens none. Reading only -- nothing that creates, deletes, \
         installs, sends or pushes, and nothing that looks like a key or a \
         password. Refusals come back with a reason; ask for what you want rather \
         than working around one.\n\
         **plan** writes down the steps and keeps them current, which is what \
         the user sees instead of a bar creeping along. Send the WHOLE list each \
         time, each step with a status of pending, active or done -- and only \
         ever ONE active, because only one thing is happening at a time. Use it \
         when the work is three or more real steps: write the plan before \
         starting, mark a step active as you begin it, done as you finish, and \
         add anything you discover along the way. Skip it for short work -- a \
         plan for a two-step task is ceremony, and the list is meant to mean \
         something.\n\
         The list is held against you. Finishing with items still pending gets \
         handed back once, and if you finish anyway your own report says which \
         ones you skipped -- so mark them done as you do them, and do not write \
         down work you are not going to do.\n\
         **read** returns part of a file with line numbers -- ask for a range \
         when it is long. Read before you edit: matching text you have not seen \
         is guessing. It reads a PDF too, whole rather than by line, because a \
         document has pages and not lines: point it at a contract or an invoice \
         and ask about what it says.\n\
         **edit** replaces one exact piece of text in a file, which is what most \
         changes actually are. What you give must appear EXACTLY ONCE, so \
         include enough around it to be unique; you are told how many matches \
         there were. Prefer this to rewriting a whole file -- a rewrite \
         regenerates everything you cannot see, and things get lost that way.\n\
         read, write and edit work on files directly and need NO editor open. \
         Launching one to write a file is the same mistake as opening Terminal \
         to run a command: a window the user did not ask for, and a slower route \
         to the same place.\n\
         **workspace** moves where commands run and files are written. Use it \
         the moment they name a folder or a project to work in -- and only then; \
         it is theirs to choose, not yours. Everything afterwards is relative to \
         it, and nothing outside it is ever touched.\n\
         **show** opens a file you made, in whatever application owns that kind \
         of file -- a page in the browser, an image in Preview. Finish with it \
         whenever you have made something meant to be looked at: writing a file \
         they cannot see is half the job, and they asked to be shown.\n\
         **write** puts a file in the workspace -- how you make something rather \
         than describe it. Creating needs no permission; replacing is refused \
         until they agree, so ask with a question naming the file. Whole files \
         only: no appending, no patching.\n\
         **open** takes a URL and launches the browser itself -- never launch one \
         first. Prefer a URL that already carries the query: one step instead of \
         four, and a step skipped is a step that cannot miss.\n\
         **launch** opens an application by name, but ONLY a name appearing \
         character for character in the list at the end. That list is the whole \
         truth about what is installed; if what they asked for is missing, it is \
         not on this Mac -- see the rule above about what to do then.\n\
         **press** sends any key combination -- cmd, shift, option, control, fn \
         with a letter, digit, punctuation or function key -- and the keys that \
         mean something alone: escape to close a dialog, tab between fields, \
         arrows and return to move and choose inside something open.\n\
         **type** puts text where the focus is. Click the field first, then type \
         on the next step. Set submit when Return should follow, which a search \
         box or address bar almost always wants.\n\
         **point** is the last resort: the SINGLE next control, the control \
         itself and not the panel around it. Only something you can actually see \
         -- if the menu is not open yet, or this is a bare desktop, say so and \
         mark it unsure. Guessing a location is worse than admitting you cannot \
         see it. Say how to reach it: click, doubleClick, or hover.\n\
         - doubleClick opens a file, folder or application from Finder or the \
           desktop; a single click there only selects.\n\
         - hover for anything inside an already-open menu: a submenu opens on \
           hover, and a click can close the menu and undo the previous step. \
           Click only the final item that performs the action.\n\n\
         Prefer a keyboard shortcut to a menu whenever one exists. Menus here are \
         unreliable to click -- they open, and the click that should choose an \
         item closes them instead. Opening a menu to READ it is fine; the \
         shortcut is printed beside each command, so read it and press that.\n\n\
         {routing}{carrying}\
         {agents}\
         ## Applications this machine can launch\n\n\
         What `launch` accepts, and nothing more. Not a list of ways to do \
         something, and not a set of alternatives to what was asked for -- if \
         what they named is absent, the answer is its website, not a substitute \
         from here.\n\n{apps}",
        goal = ask.goal,
        workspace = ask.workspace,
        facts = ask.facts.brief(),
        controls = controls_here(ask.controls),
        apps = crate::core::screen::launch::installed_apps().join(", "),
        agents = agents_here(),
    )
}

/// The controls macOS says are on screen, numbered so one can be named exactly.
///
/// The point of this block is that it removes a guess. Finding a button in a
/// picture is the single least reliable thing the model does -- measured at
/// roughly two thirds, with the misses landing 12 to 87 pixels out. Choosing a
/// row from a list is a different kind of task, and an easier one.
///
/// Capped, and ordered down the screen so the list reads the way the window
/// does. A window with four hundred controls in it is a list nobody can choose
/// from, including a model.
fn controls_here(controls: &[crate::core::screen::ax::Control]) -> String {
    if controls.is_empty() {
        return String::new();
    }
    let shown = ordered(controls);
    let lines: Vec<String> = shown
        .iter()
        .enumerate()
        .map(|(i, c)| format!("{}. {} \"{}\"", i + 1, role_word(&c.role), c.label))
        .collect();
    let more = match controls.len() > shown.len() {
        true => format!("\n(and {} more, not listed)", controls.len() - shown.len()),
        false => String::new(),
    };
    format!(
        "The system reports these controls on screen, already located exactly. \
         To act on one, use its number -- that is always better than pointing at \
         a pixel, because the number cannot miss. If what you want is not here, \
         point at it in the picture as usual; this list is what the application \
         chose to expose, not everything that exists.\n\n{}{more}\n\n",
        lines.join("\n")
    )
}

/// The controls in the order they are numbered, and only the ones numbered.
///
/// The single source of that order. The prompt numbers this list and a reply
/// names a number, and the two have to be the same list or a reply means
/// something else entirely -- click the wrong row, confidently, with the model
/// blameless. Two functions sorting "the same way" is exactly the kind of
/// agreement that lasts until someone changes one of them.
///
/// Ordered down the screen so the list reads the way the window does, and capped
/// because a list of four hundred is not a list anyone can choose from.
fn ordered(
    controls: &[crate::core::screen::ax::Control],
) -> Vec<&crate::core::screen::ax::Control> {
    const MAX: usize = 60;
    let mut out: Vec<&crate::core::screen::ax::Control> = controls.iter().collect();
    out.sort_by_key(|c| (c.at.1 as i64, c.at.0 as i64));
    out.truncate(MAX);
    out
}

/// The control a reply means by `control: n`, counting from one as the prompt does.
pub fn control_at(
    controls: &[crate::core::screen::ax::Control],
    n: u64,
) -> Option<&crate::core::screen::ax::Control> {
    ordered(controls).get(n.checked_sub(1)? as usize).copied()
}

/// The role, in a word someone would use out loud.
fn role_word(role: &str) -> &str {
    // The prefix is a macOS habit, not a meaning. UI Automation answers "Button"
    // where AX answers "AXButton", and the model should be shown the same word
    // either way -- it is describing what is on screen, not which framework
    // reported it.
    match role.strip_prefix("AX").unwrap_or(role) {
        "Button" => "button",
        "MenuItem" | "MenuBarItem" => "menu",
        "CheckBox" => "checkbox",
        "RadioButton" => "option",
        "PopUpButton" => "dropdown",
        "TextField" | "TextArea" => "text field",
        "Link" => "link",
        "Tab" => "tab",
        "Row" | "Cell" => "row",
        "DisclosureTriangle" => "twisty",
        other => other,
    }
}

/// The steps so far, newest kept whole and older ones cut to their first line.
///
/// Everything a step produced used to be carried forever: a `read` of a 38,000
/// character file, a fetched page, a command's output -- re-sent on every turn
/// after it. Measured: the prompt was 17,000 characters at the start of a task
/// and 70,000 three turns in, which is most of why a live turn took twelve
/// seconds when the same model answered the same screenshots in four.
///
/// What the model actually needs differs by age. The last thing it did, it needs
/// in full -- that is the result it is reasoning about. What it did five turns
/// ago it needs to *remember doing*, so it does not do it again; the contents
/// are long since spent.
fn recent(done: &[String]) -> String {
    /// Roughly a page of text. Enough for the last file read or page fetched,
    /// and bounded so a long task does not slow down as it goes.
    const BUDGET: usize = 8_000;

    if done.is_empty() {
        return "Nothing yet.".to_string();
    }
    let mut lines: Vec<String> = Vec::with_capacity(done.len());
    let mut spent = 0usize;
    // Newest first, so the budget is spent on what matters most.
    for (i, step) in done.iter().enumerate().rev() {
        let n = i + 1;
        let room = BUDGET.saturating_sub(spent);
        if step.len() <= room {
            spent += step.len();
            lines.push(format!("{n}. {step}"));
            continue;
        }
        // Past the budget: what was done, not what it said.
        let first = step.lines().next().unwrap_or("");
        let head: String = first.chars().take(120).collect();
        lines.push(format!(
            "{n}. {head}{}",
            if step.len() > first.len() || first.len() > 120 {
                " … (output no longer shown)"
            } else {
                ""
            }
        ));
    }
    lines.reverse();
    lines.join("\n")
}

/// Guide mode: someone is watching, and some requests are not tasks at all.
const GUIDE_ROUTING: &str = "## Is this even a task?\n\n\
     Not every request is a task. If they are chatting, greeting you, asking \
     about you, or asking something the screen cannot answer, just reply. Do not \
     invent a control to point at so you have something to do. If it was never a \
     task, reply and leave it there.\n\n\
     Count the actions the goal needs. If finishing it takes MORE THAN ONE, it is \
     agent work: answer with agent and a short title, and do NOT answer with the \
     first of those actions. Launching an application or opening a page is almost \
     never the whole job, and anything ending in something played, sent, created \
     or found is more. Only a goal genuinely finished by one action is that one \
     action. Choose agent when they want the outcome; choose point when they want \
     to know where something is.\n\n\
     Set background only when the task will take minutes and they should get on \
     with something else meanwhile. Anything over in a few seconds is not \
     background.\n\n";

/// Agent mode: nobody is watching, so the rules are about restraint.
const AGENT_ROUTING: &str = "## You are the one acting\n\n\
     You are carrying this out YOURSELF, right now, with nobody watching. Never \
     answer with agent -- you already are one. Every turn must move the task \
     forward, or finish it.\n\n\
     reply ENDS the task, so use it only to give up and say why. If you need \
     them to DO something before you can carry on -- scan a code, sign in, \
     unlock something -- that is a question, not a reply: a question is spoken \
     and then waits for their answer, and the task carries on from there with \
     everything you have done so far still in place.\n\n";

const AGENT_RULES: &str = "## Knowing when you are done\n\n\
     Decide first whether the goal is ALREADY met, and say so in `screen`. Most \
     controls toggle -- play also pauses, mute also unmutes -- so acting on a \
     goal that is already met undoes it. A control listed in the steps above has \
     been used: do not use it again.\n\n\
     Trust the system report over the picture, read for what it says. Audio \
     playing means this Mac is making a sound, not that it is the sound you were \
     asked for; the window title tells you whose it is. When the two agree, THE \
     GOAL IS MET -- say done and stop. Do not take one more action to be sure: \
     what you did last turn worked, and the control you are reaching for is the \
     one that undoes it. A still frame cannot show motion, so never conclude from \
     the picture that nothing is happening when the report says it is. If the \
     report names a different frontmost application than you think you are \
     looking at, it is right and your view is stale or covered.\n\n\
     If the same action appears twice in the steps above it did not work, and a \
     third will not either: try a different control, a different route, or say \
     you are stuck.\n\n\
     If the goal was a QUESTION, the answer is words, and `say` is spoken aloud \
     -- so put it there. The temperature, the price, the number they asked for. \
     Getting the answer on screen is the middle of the job, not the end.\n\n\
     ## Doing only what was asked\n\n\
     Do exactly what was asked and then STOP -- not the helpful next thing. When \
     a goal could be read narrowly or broadly, TAKE THE NARROW ONE and offer the \
     rest as `next`. Starting something is finished when the thing exists, not \
     when it has content. Setting something up is finished when it is set up, not \
     configured. The broad reading always involves choices they did not give you, \
     made on their behalf with their cursor, on their real machine.\n\n\
     Clearing what is IN THE WAY is part of the job: a cookie banner, an upgrade \
     prompt, a sign-in popup over the thing you need. Dismiss those and carry on. \
     Never sign in, buy, or agree to anything.\n\n\
     NEVER open a file picker or browser for a goal that named no file, and never \
     pick a file inside one. That dialog is a decision about their own documents. \
     If the task truly needs a file and none was named, ask which.\n\n\
     ## Asking\n\n\
     BEFORE YOUR FIRST ACTION, work out everything this task needs that you were \
     not told -- who it goes to, what it should say, which one they meant, how \
     much -- and ask for ALL of it in one natural spoken question.\n\
     Make the question easy to answer. Offer a couple of concrete examples of \
     what an answer could look like, so they can pick one instead of composing \
     something: not \"what sections do you want?\" but \"what is it for, and what \
     should be on it? A portfolio with about, projects and contact, say, or a \
     shop with products and a booking form.\" Then say what you will do with the \
     answer -- that you will start as soon as they tell you -- so the question \
     reads as the last thing before the work rather than an obstacle in front of \
     it. Ask on your \
     very first turn, before opening anything: stopping halfway to ask something \
     you could have asked at the start wastes the time between and leaves \
     half-finished work on screen. Guessing is worse than asking; a message sent \
     to the wrong person cannot be taken back.\n\n\
     After that, ask only about what you could NOT have known in advance -- a \
     login only they can complete, or which of several matches they meant when \
     you could not have known there were several. Never ask to confirm what they \
     already said, and never ask for what you could read off the screen.\n\n\
     ## Finishing\n\n\
     When you finish you may add `next` to the done answer: ONE short spoken \
     offer of the obvious next step, phrased as an invitation rather than a \
     question -- beginning with something like *if you want, I can*, rather than \
     asking whether you shall. They have what they asked for either way, and an \
     invitation can be ignored without it feeling like something was left \
     hanging. Only when there genuinely is one \
     and they would plausibly want it now -- typically the broad reading you \
     deliberately stopped short of. Most tasks end with nothing worth asking: \
     leave `next` out. Never offer for the sake of it, never ask whether there is \
     anything else, and never offer what you just did. An agent that asks every \
     time is worse than one that never does, because then the question means \
     nothing.\n\n";

/// Models wrap JSON in prose and code fences no matter how firmly you ask.
/// The one object to act on, out of whatever the model actually sent.
///
/// **A list is the interesting case.** Asked for the next step, a model that has
/// worked out four of them sometimes sends all four as a JSON array. That is a
/// reasonable thing to do and Nudge performs one step per turn, so the right
/// answer is the first of them -- not, as this used to do, nothing at all.
///
/// It failed by accident rather than by rule: the span from the first `{` to the
/// last `}` is `{...},{...}` for a two-element array, which is not JSON, so a
/// perfectly good first step was discarded and the turn was spent saying "no
/// usable point". Seen in a real run, where a write of a finished document was
/// thrown away because a plan came with it.
pub(crate) fn first_json(text: &str) -> Option<serde_json::Value> {
    let trimmed = text
        .trim()
        .trim_start_matches("```json")
        .trim_matches('`')
        .trim();

    // Whole-reply first, which is what a well-behaved model sends and what
    // handles a list correctly.
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
        return match v {
            serde_json::Value::Array(items) => items.into_iter().find(|i| i.is_object()),
            v if v.is_object() => Some(v),
            _ => None,
        };
    }

    // A list with prose around it, or after it.
    if let (Some(open), Some(close)) = (trimmed.find('['), trimmed.rfind(']')) {
        if open < close {
            if let Ok(serde_json::Value::Array(items)) =
                serde_json::from_str::<serde_json::Value>(&trimmed[open..=close])
            {
                if let Some(first) = items.into_iter().find(|i| i.is_object()) {
                    return Some(first);
                }
            }
        }
    }

    // One object with prose around it, which is the ordinary untidy case.
    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;
    serde_json::from_str(trimmed.get(start..=end)?).ok()
}

/// Unknown or missing means click: it is what most targets want, and a wrong
/// click is recoverable where a wrong hover just stalls.
/// Shared by the JSON providers: the outcomes that carry no coordinates.
pub(crate) fn simple_step(kind: &str, v: &serde_json::Value, say: String) -> Option<Step> {
    // Set by the model when the sentence states something it is remembering
    // rather than something it just saw, ran or read. Applied here, at the one
    // place both outcomes are built, so everything downstream -- spoken, shown,
    // written into the history -- carries it without knowing about it.
    let say = match v["recalled"].as_bool().unwrap_or(false) {
        true => recalled(say),
        false => say,
    };
    match kind {
        "done" => Some(Step::Done {
            say,
            // Blank or missing means no offer, which is the common case.
            next: v["next"]
                .as_str()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string),
        }),
        "unsure" => Some(Step::Unsure {
            say,
            needed: v["needed"]
                .as_str()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string),
        }),
        "reply" => Some(Step::Reply { say }),
        "skill" => Some(Step::Skill {
            name: v["name"].as_str().unwrap_or_default().to_string(),
            say,
        }),
        "remember" => Some(Step::Remember {
            about: v["about"].as_str().unwrap_or_default().to_string(),
            note: v["note"].as_str().unwrap_or_default().to_string(),
            say,
        }),
        "delegate" => Some(Step::Delegate {
            task: v["task"].as_str().unwrap_or_default().to_string(),
            named: v["named"]
                .as_str()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string),
            say,
        }),
        "request" => Some(Step::Request {
            method: v["method"].as_str().unwrap_or("GET").to_string(),
            url: v["url"].as_str().unwrap_or_default().to_string(),
            // An object, because that is how anyone writes headers. Values are
            // taken as text whatever they arrived as -- a number in a header is
            // still a header.
            headers: v["headers"]
                .as_object()
                .map(|h| {
                    h.iter()
                        .map(|(k, v)| {
                            let value = match v.as_str() {
                                Some(s) => s.to_string(),
                                None => v.to_string(),
                            };
                            (k.clone(), value)
                        })
                        .collect()
                })
                .unwrap_or_default(),
            body: match &v["body"] {
                serde_json::Value::Null => None,
                serde_json::Value::String(s) => Some(s.clone()),
                // An object handed straight through as JSON, which is what was
                // meant -- writing it out as a string first is a step the model
                // gets wrong more often than not.
                other => Some(other.to_string()),
            },
            say,
        }),
        "mcp" => Some(Step::Mcp {
            tool: v["tool"].as_str().unwrap_or_default().to_string(),
            // Missing means no arguments, which plenty of tools take.
            args: match v["args"].is_null() {
                true => serde_json::json!({}),
                false => v["args"].clone(),
            },
            say,
        }),
        "launch" => Some(Step::Launch {
            app: v["app"].as_str().unwrap_or_default().to_string(),
            say,
        }),
        "open" => Some(Step::Open {
            url: v["url"].as_str().unwrap_or_default().to_string(),
            say,
        }),
        "type" => Some(Step::Type {
            text: v["text"].as_str().unwrap_or_default().to_string(),
            submit: v["submit"].as_bool().unwrap_or(false),
            say,
        }),
        "plan" | "todo" => Some(Step::Plan {
            todos: v["todos"]
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .map(|t| {
                            (
                                t["text"].as_str().unwrap_or_default().to_string(),
                                t["status"].as_str().unwrap_or("pending").to_string(),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default(),
            say,
        }),
        "read" => Some(Step::Read {
            path: v["path"].as_str().unwrap_or_default().to_string(),
            from: v["from"].as_u64().unwrap_or(1) as usize,
            lines: v["lines"].as_u64().unwrap_or(0) as usize,
            say,
        }),
        "edit" => Some(Step::Edit {
            path: v["path"].as_str().unwrap_or_default().to_string(),
            old: v["old"].as_str().unwrap_or_default().to_string(),
            new: v["new"].as_str().unwrap_or_default().to_string(),
            say,
        }),
        "workspace" => Some(Step::Workspace {
            path: v["path"].as_str().unwrap_or_default().to_string(),
            say,
        }),
        "show" => Some(Step::Show {
            path: v["path"].as_str().unwrap_or_default().to_string(),
            say,
        }),
        "task" => Some(Step::Task {
            task: v["task"].as_str().unwrap_or_default().to_string(),
            say,
        }),
        "search" => Some(Step::Search {
            query: v["query"].as_str().unwrap_or_default().to_string(),
            say,
        }),
        "fetch" => Some(Step::Fetch {
            url: v["url"].as_str().unwrap_or_default().to_string(),
            say,
        }),
        "write" => Some(Step::Write {
            path: v["path"].as_str().unwrap_or_default().to_string(),
            content: v["content"].as_str().unwrap_or_default().to_string(),
            say,
        }),
        "start" => Some(Step::Start {
            command: v["command"].as_str().unwrap_or_default().to_string(),
            say,
        }),
        "await" => Some(Step::Await {
            id: v["id"].as_u64()?,
            say,
        }),
        "output" => Some(Step::Output {
            id: v["id"].as_u64().unwrap_or(0),
            say,
        }),
        "kill" => Some(Step::Kill {
            id: v["id"].as_u64().unwrap_or(0),
            say,
        }),
        "run" => Some(Step::Run {
            command: v["command"].as_str().unwrap_or_default().to_string(),
            say,
        }),
        "press" => Some(Step::Press {
            keys: v["keys"].as_str().unwrap_or_default().to_string(),
            say,
        }),
        "agent" => Some(Step::Agent {
            title: v["title"].as_str().unwrap_or("Working").to_string(),
            say,
            // Absent means foreground. A card that appears when it was not asked
            // for is worse than one that never appears.
            background: v["background"].as_bool().unwrap_or(false),
        }),
        "ask" | "question" => Some(Step::Question {
            question: v["question"].as_str().unwrap_or(&say).to_string(),
        }),
        _ => None,
    }
}

pub(crate) fn act_from(raw: Option<&str>) -> Act {
    match raw.unwrap_or("") {
        "doubleClick" | "double_click" | "double" => Act::DoubleClick,
        "hover" | "mouse_move" | "move" => Act::Hover,
        _ => Act::Click,
    }
}

pub(crate) fn no_point(provider: &'static str, detail: impl Into<String>) -> Error {
    Error::NoPoint {
        provider,
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests {
    /// Handing the same job over twice is the same action.
    ///
    /// These fell through to "not a repeat", which is how four identical
    /// delegations to a coding agent went through unnoticed.
    #[test]
    fn handing_the_same_job_over_twice_is_a_repeat() {
        let hand = |task: &str| Step::Delegate {
            task: task.into(),
            named: None,
            say: String::new(),
        };
        assert!(hand("research it").same_action(&hand("research it")));
        assert!(!hand("research it").same_action(&hand("write it up")));

        let ask = |task: &str| Step::Task {
            task: task.into(),
            say: String::new(),
        };
        assert!(ask("find the version").same_action(&ask("find the version")));
        assert!(!ask("find the version").same_action(&ask("find the author")));
    }

    /// Planning twice in a row is no progress, however it is worded.
    #[test]
    fn two_plans_in_a_row_are_a_repeat() {
        let plan = |first: &str| Step::Plan {
            todos: vec![(first.into(), "active".into())],
            say: String::new(),
        };
        // Different words, different lists, same absence of work.
        assert!(plan("research the frameworks").same_action(&plan("look into the frameworks")));

        // But a plan after doing something is an update, which is the point of
        // keeping one.
        let did = Step::Run {
            command: "ls".into(),
            say: String::new(),
        };
        assert!(!did.same_action(&plan("research the frameworks")));
    }

    /// Same tool and same arguments is a repeat; different arguments is work.
    #[test]
    fn a_tool_called_twice_the_same_way_is_a_repeat() {
        let call = |path: &str| Step::Mcp {
            tool: "files/read_file".into(),
            args: serde_json::json!({ "path": path }),
            say: String::new(),
        };
        assert!(call("a.md").same_action(&call("a.md")));
        // A run reading five files calls one tool five times and is working.
        assert!(!call("a.md").same_action(&call("b.md")));
    }

    /// A model that plans ahead sends a list. The first of them is the step.
    ///
    /// Found in a real run: a finished document was written, the model sent the
    /// write together with what it meant to do next, and the whole turn was
    /// discarded as "no usable point".
    #[test]
    fn a_plan_arriving_as_a_list_still_yields_a_step() {
        let v = super::first_json(
            r#"[{"kind":"write","path":"a.md","content":"x","say":"writing"},
                {"kind":"read","path":"a.md","say":"checking"}]"#,
        )
        .expect("a list is not nothing");
        assert_eq!(v["kind"], "write");
        assert_eq!(v["path"], "a.md");
    }

    #[test]
    fn a_list_with_prose_around_it_still_works() {
        let v = super::first_json(
            "Here is my plan:\n[{\"kind\":\"done\",\"say\":\"finished\"}]\nHope that helps.",
        )
        .expect("prose is not a reason to lose the step");
        assert_eq!(v["kind"], "done");
    }

    /// The ordinary case, and a fenced one.
    #[test]
    fn a_single_object_is_read_however_it_is_wrapped() {
        for reply in [
            r#"{"kind":"done","say":"ok"}"#,
            "```json\n{\"kind\":\"done\",\"say\":\"ok\"}\n```",
            "Sure.\n{\"kind\":\"done\",\"say\":\"ok\"}",
        ] {
            assert_eq!(
                super::first_json(reply).expect(reply)["kind"],
                "done",
                "{reply}"
            );
        }
    }

    /// An empty list is not a step, and neither is prose.
    #[test]
    fn nothing_usable_is_still_nothing() {
        assert!(super::first_json("[]").is_none());
        assert!(super::first_json("I am not sure what to do.").is_none());
        assert!(super::first_json("").is_none());
    }

    use super::*;

    #[test]
    fn digs_json_out_of_fences_and_chatter() {
        let v = first_json("Sure!\n```json\n{\"x\": 12, \"y\": 7}\n```\nHope that helps")
            .expect("should find the object");
        assert_eq!(v["x"], 12);
        assert_eq!(v["y"], 7);
        assert!(first_json("no object here").is_none());
    }

    /// The same control, described the same way, whichever system reported it.
    ///
    /// macOS says "AXButton" and UI Automation says "Button". If those reach the
    /// model as different words it will reason about them as different things,
    /// and a prompt that was tuned on one platform quietly means something else
    /// on the other.
    #[test]
    fn a_button_is_a_button_on_every_platform() {
        for (mac, elsewhere) in [
            ("AXButton", "Button"),
            ("AXMenuItem", "MenuItem"),
            ("AXTextField", "TextField"),
            ("AXRow", "Row"),
            ("AXLink", "Link"),
        ] {
            assert_eq!(
                role_word(mac),
                role_word(elsewhere),
                "{mac} and {elsewhere} are the same control described differently"
            );
        }
        // And anything neither list has arrives unmangled rather than as a guess.
        assert_eq!(role_word("Thermostat"), "Thermostat");
    }

    /// The prompt numbers a list and the reply names a number. If those two
    /// disagree by even one, every reply clicks the wrong control and nothing
    /// looks wrong -- the model named something real, we clicked something real,
    /// and the only symptom is that it is the wrong thing.
    ///
    /// So this walks the rendered block line by line and checks that the label
    /// on line n is the control `control_at(n)` hands back.
    #[test]
    fn the_number_the_model_is_shown_is_the_control_we_click() {
        use crate::core::screen::ax::Control;
        let c = |label: &str, x: f64, y: f64| Control {
            role: "AXButton".into(),
            label: label.into(),
            at: (x, y),
            size: (10.0, 10.0),
        };
        // Deliberately not in reading order: the whole job of `ordered` is to
        // put them in one, and a list that arrived sorted would prove nothing.
        let controls = vec![
            c("Bottom left", 10.0, 900.0),
            c("Top right", 900.0, 10.0),
            c("Top left", 10.0, 10.0),
            c("Middle", 400.0, 400.0),
        ];

        let block = controls_here(&controls);
        let numbered: Vec<&str> = block
            .lines()
            .filter(|l| l.starts_with(|ch: char| ch.is_ascii_digit()))
            .collect();
        assert_eq!(
            numbered.len(),
            controls.len(),
            "not every control was listed"
        );

        for (i, line) in numbered.iter().enumerate() {
            let n = i as u64 + 1;
            assert!(line.starts_with(&format!("{n}. ")), "line {n} is {line:?}");
            let resolved = control_at(&controls, n).expect("a control for every line shown");
            assert!(
                line.contains(&resolved.label),
                "line {n} says {line:?} but control {n} is {:?}",
                resolved.label
            );
        }

        // Reading order, top to bottom then left to right.
        assert!(numbered[0].contains("Top left"));
        assert!(numbered[1].contains("Top right"));
        assert!(numbered[3].contains("Bottom left"));

        // And the ways a number can mean nothing.
        assert!(control_at(&controls, 0).is_none(), "the list starts at one");
        assert!(control_at(&controls, 5).is_none(), "past the end");
        assert_eq!(
            controls_here(&[]),
            "",
            "no controls, no heading about controls"
        );
    }

    fn ask<'a>(goal: &'a str, done: &'a [String], stalled: bool) -> Ask<'a> {
        Ask {
            goal,
            done,
            stalled,
            agent: false,
            facts: Default::default(),
            controls: &[],
            tools: &[],
            reach: String::new(),
            shell: false,
            memory: String::new(),
            earlier: &[],
            skills: String::new(),
            workspace: "/tmp/workspace".into(),
        }
    }

    /// The bug this guards: the agent was handed guide mode's prompt, which
    /// offers "answer with agent for a whole job". So on turn one it answered
    /// `agent` -- delegating the task to itself -- and nothing ever happened.
    #[test]
    fn an_agent_is_never_told_it_can_hand_the_job_to_an_agent() {
        let mut a = ask("play bohemian rhapsody on youtube", &[], false);
        a.agent = true;
        let p = prompt(&a);
        // Matched on the routing sentence itself. "answer with agent" is too
        // loose -- the agent prompt says "Never answer with agent", and a
        // substring test cannot tell a prohibition from an offer.
        assert!(!p.contains("it is agent work"), "offered itself a delegate");
        assert!(
            !p.contains("Not every request is a task"),
            "offered to chat at nobody"
        );
        assert!(p.contains("YOURSELF"), "never told it is the one acting");
        assert!(
            p.contains("Never answer with agent"),
            "not forbidden from forking"
        );

        // Guide mode still gets both -- there is a person reading it.
        let g = prompt(&ask("play bohemian rhapsody on youtube", &[], false));
        assert!(
            g.contains("it is agent work"),
            "guide mode lost its routing"
        );
        assert!(
            g.contains("Not every request is a task"),
            "guide mode lost its chat path"
        );
        assert!(
            !g.contains("YOURSELF"),
            "guide mode told it is acting alone"
        );
    }

    /// Nineteen turns of one run were spent clicking the same YouTube link while
    /// the song was already playing. The guard that should have caught it
    /// compared sentences, and the model rewords every single turn.
    #[test]
    fn the_same_action_is_recognised_through_different_words() {
        let here = |x: f64, y: f64, say: &str| Step::Point {
            control: None,
            at: Point { x, y },
            say: say.into(),
            act: Act::Click,
        };
        // Re-aiming at one link drifts a few pixels a turn.
        assert!(
            here(1106.0, 385.0, "Let's hit play on that first track").same_action(&here(
                1115.0,
                386.0,
                "Let's click that first track in the sidebar"
            ))
        );
        // A genuinely different control is not a repeat.
        assert!(!here(1106.0, 385.0, "a").same_action(&here(1106.0, 460.0, "a")));

        let url = |u: &str, say: &str| Step::Open {
            url: u.into(),
            say: say.into(),
        };
        assert!(url("https://youtube.com/x", "Heading straight to YouTube")
            .same_action(&url("https://YouTube.com/X", "Let's teleport to YouTube")));
        assert!(!url("https://youtube.com/x", "a").same_action(&url("https://youtube.com/y", "a")));
    }

    /// Told only the sentence, the model cannot tell that five turns aimed at the
    /// same pixel and none of them worked.
    #[test]
    fn history_says_where_it_clicked() {
        // And says *what* it clicked when the system named it. A coordinate is
        // what we had to write down when a pixel was all we knew; a name is what
        // the model can actually reason about next turn.
        let named = Step::Point {
            control: Some("Send".into()),
            at: Point { x: 10.0, y: 20.0 },
            say: "Sending it".into(),
            act: Act::Click,
        };
        let line = named.recap();
        assert!(line.contains("Send"), "the name is missing from {line:?}");
        assert!(!line.contains("10"), "a name beats a coordinate: {line:?}");

        let s = Step::Point {
            control: None,
            at: Point {
                x: 1106.4,
                y: 385.9,
            },
            say: "Let's hit play".into(),
            act: Act::Click,
        };
        assert_eq!(s.recap(), "Click at (1106, 386) -- Let's hit play");
    }

    /// Rules that were replaced must actually be gone.
    ///
    /// The absolute "never launch an application to look something up" was
    /// rewritten as an ordering, because there are facts no fetch can reach --
    /// their mail, their calendar. The replacement went in and the original
    /// stayed, four lines above it, so the prompt carried both and the wrong one
    /// won: a run opened the Weather app to read one number and then failed
    /// trying to drive it.
    #[test]
    fn no_rule_survives_the_rule_that_replaced_it() {
        for mode in [true, false] {
            let mut a = ask("what is the weather", &[], false);
            a.agent = mode;
            let p = prompt(&a);
            assert!(
                !p.contains("Never launch an application to look something up"),
                "the absolute survived its own replacement (agent = {mode})"
            );
            assert!(
                p.contains("cheapest thing that can ACTUALLY answer"),
                "and the ordering that replaced it is missing"
            );
            // Both halves, or it becomes an absolute again by omission.
            assert!(p.contains("is on the web: fetch it"));
            assert!(p.contains("not on the web at all"));
        }
    }

    /// Each tool is described once. Saying the same thing twice in one prompt is
    /// how two versions of a rule drift apart.
    /// No section may contain a literal backslash-n.
    ///
    /// Written after making the same mistake twice in one sitting: a prompt
    /// fragment built with `\\n` in the source produces the two characters rather
    /// than a newline, and the whole section arrives as one unbroken line reading
    /// `## Skills\\n\\nThings this person...`. It still half-works, which is why
    /// nothing catches it -- the model copes and nobody looks at the prompt.
    #[test]
    fn no_section_has_escaped_newlines_in_it() {
        let done = ["Opened Safari".to_string()];
        let mut a = ask("do something", &done, false);
        let tools = [crate::core::tools::mcp::Tool {
            server: "files".into(),
            name: "read".into(),
            about: "Reads a file.".into(),
            schema: serde_json::json!({}),
        }];
        let before = ["They had asked: x".to_string()];
        a.tools = &tools;
        a.earlier = &before;
        a.memory = crate::core::memory::Memory::default().prompt(None);
        a.skills = crate::core::skills::prompt();
        a.reach = crate::core::reach::Reach::default().prompt();

        let p = prompt(&a);
        assert!(
            !p.contains("\\n"),
            "a prompt section contains a literal backslash-n"
        );
    }

    #[test]
    fn nothing_is_said_twice() {
        let mut a = ask("x", &[], false);
        a.agent = true;
        let p = prompt(&a);
        for once in [
            "A still frame cannot show motion",
            "never launch one first",
            "TAKE THE NARROW ONE",
            "cheapest thing that can ACTUALLY answer",
        ] {
            assert_eq!(
                p.matches(once).count(),
                1,
                "{once:?} appears more than once"
            );
        }
    }

    /// What the model needs differs by age: the last thing it did in full,
    /// because that is the result it is reasoning about; older steps only as a
    /// reminder that it did them, so it does not do them twice.
    #[test]
    fn the_newest_step_survives_whole_and_older_ones_shrink() {
        let done = vec![
            format!("Read a.rs:\n{}", "a".repeat(30_000)),
            format!("Read b.rs:\n{}", "b".repeat(30_000)),
            "Ran `ls`, which printed:\nCargo.toml\nsrc".to_string(),
        ];
        let out = recent(&done);

        assert!(out.contains("Cargo.toml"), "the newest lost its contents");
        assert!(
            !out.contains(&"a".repeat(200)),
            "an old file is still in full"
        );
        assert!(
            !out.contains(&"b".repeat(200)),
            "an old file is still in full"
        );

        // Still remembers doing them, in order, so it does not repeat them.
        assert!(out.contains("1. Read a.rs"));
        assert!(out.contains("2. Read b.rs"));
        assert!(out.contains("3. Ran `ls`"));
        let (a, b) = (out.find("1. Read a.rs"), out.find("3. Ran"));
        assert!(a < b, "history came back out of order");

        // And it says the output is gone rather than implying there was none.
        assert!(out.contains("no longer shown"));
    }

    #[test]
    fn an_empty_history_says_so() {
        assert_eq!(recent(&[]), "Nothing yet.");
    }

    /// How big is what actually goes out, and what is it made of?
    ///
    ///     cargo test what_the_prompt_costs -- --nocapture
    #[test]
    fn what_the_prompt_costs() {
        let empty = prompt(&ask("do a thing", &[], false));
        let mut a = ask("do a thing", &[], false);
        a.agent = true;
        let agent_mode = prompt(&a);

        // What a few turns of real work leaves behind: a file read, a page
        // fetched, a command's output.
        let history: Vec<String> = vec![
            format!("Read src/main.rs:\n{}", "x".repeat(38_000)),
            format!(
                "Read https://example.com, which says:\n{}",
                "y".repeat(11_000)
            ),
            format!("Ran `cargo test`, which printed:\n{}", "z".repeat(3_800)),
        ];
        let mut b = ask("do a thing", &history, false);
        b.agent = true;
        let with_history = prompt(&b);

        eprintln!("guide mode, no history : {:>7} chars", empty.len());
        eprintln!("agent mode, no history : {:>7} chars", agent_mode.len());
        eprintln!("agent mode, 3 turns in : {:>7} chars", with_history.len());
        eprintln!(
            "   of which history     : {:>7} chars",
            with_history.len() - agent_mode.len()
        );
    }

    #[test]
    fn a_recalled_answer_is_marked_where_it_is_parsed() {
        let v = serde_json::json!({ "recalled": true });
        let step = simple_step("done", &v, "macOS 27 shipped in 2026.".into()).unwrap();
        assert!(step.say().starts_with(super::RECALLED));

        // The common case is unmarked, including ordinary conversation.
        let plain = simple_step("reply", &serde_json::json!({}), "Morning.".into()).unwrap();
        assert_eq!(plain.say(), "Morning.");
    }

    /// Not the same claim. "I did not check this" is not "I do not know", and a
    /// harness that treated the marker as a hedge would score an unchecked wrong
    /// answer as an honest one.
    #[test]
    fn the_recalled_marker_is_not_a_hedge() {
        assert!(!crate::core::run::subagent::hedged(super::RECALLED));
    }

    #[test]
    fn the_prompt_explains_when_to_mark_a_fact_as_recalled() {
        let p = prompt(&ask("when did macOS 27 ship?", &[], false));
        assert!(p.contains("recalled: true"));
    }

    /// Listing a tool is not the same as telling the model to prefer it.
    ///
    /// The live failure this is for: asked to read a shopping list it used the
    /// file tools, and asked to *add* to it, it opened Reminders and typed into a
    /// suggested template. The tools were in the prompt and missing from the
    /// ordering, and the ordering is the part that decides.
    #[test]
    fn tools_are_in_the_ordering_not_only_in_the_list() {
        let tool = crate::core::tools::mcp::Tool {
            server: "files".into(),
            name: "write_file".into(),
            about: "Write a file.".into(),
            schema: serde_json::json!({}),
        };
        let tools = [tool];
        let mut a = ask("add bread to my shopping list", &[], false);
        a.tools = &tools;
        let p = prompt(&a);
        assert!(p.contains("a tool on a connected server; a command"));
        // And the line that used to send it to the screen for the user's own
        // files must no longer call that the only route.
        assert!(!p.contains("the only honest route"));
        assert!(p.contains("reading or changing"));

        // With nothing connected the old wording stands, unchanged.
        let none = prompt(&ask("x", &[], false));
        assert!(none.contains("the only honest route"));
        assert!(!none.contains("a tool on a connected server"));
    }

    #[test]
    fn tools_are_listed_only_when_there_are_some() {
        let quiet = prompt(&ask("do a thing", &[], false));
        assert!(!quiet.contains("Tools on connected servers"));

        let tool = crate::core::tools::mcp::Tool {
            server: "files".into(),
            name: "read_text_file".into(),
            about: "Read a file from disk.".into(),
            schema: serde_json::json!({
                "properties": {"path": {"type": "string"}},
                "required": ["path"],
            }),
        };
        let mut a = ask("read my notes", &[], false);
        let tools = [tool];
        a.tools = &tools;
        let loud = prompt(&a);
        assert!(loud.contains("files/read_text_file(path*)"));
        assert!(loud.contains("Tools on connected servers"));
    }

    /// 4.5, and the distinction the whole thing turns on: what happened a moment
    /// ago must not read as progress towards what was just asked.
    #[test]
    fn a_warm_thread_is_kept_apart_from_work_on_this_goal() {
        let quiet = prompt(&ask("open safari", &[], false));
        assert!(!quiet.contains("A moment ago"), "said with nothing to say");

        let mut a = ask("now go to wikipedia", &[], false);
        let before = [
            "They had asked: open safari".to_string(),
            "Opened Safari".to_string(),
        ];
        a.earlier = &before;
        let p = prompt(&a);
        assert!(p.contains("A moment ago"));
        assert!(p.contains("Opened Safari"));
        // Said plainly, because a model that reads this as work done will report
        // a goal finished that it never started.
        assert!(p.contains("none of it counts towards the goal"));
        // And the goal is still the new one.
        assert!(p.contains("now go to wikipedia"));
    }

    #[test]
    fn prompt_numbers_completed_steps() {
        let p = prompt(&ask("unwrap UVs", &["Opened the UV editor".into()], false));
        assert!(p.contains("1. Opened the UV editor"));
        assert!(prompt(&ask("x", &[], false)).contains("Nothing yet."));
    }

    /// The inverse of what this used to assert, which is the whole of 3.5.
    ///
    /// It used to check that every installed agent appeared by name, with its
    /// exact invocation, so the model could match what somebody said against a
    /// list and copy a command line. That is a choice being presented -- and a
    /// list of product names in the prompt is a guarantee that those names come
    /// back out of the assistant's mouth. Somebody who has never heard of a
    /// coding agent should be able to install one and never learn it exists.
    #[test]
    fn the_prompt_never_names_a_coding_agent() {
        let p = prompt(&ask("refactor this", &[], false));
        let here = crate::core::tools::running::agents_installed();

        // The invocation is the unambiguous half: if no command form appears
        // anywhere, the model cannot be composing one.
        for (name, _, form, _) in &here {
            assert!(!p.contains(form), "the prompt carries {name}'s invocation");
        }

        match here.is_empty() {
            true => assert!(!p.contains("Whole jobs"), "offered with nothing to offer"),
            false => {
                let lo = p.find("## Whole jobs").expect("no way to hand a job over");
                let section = &p[lo..p[lo..].find("\n\n## ").map_or(p.len(), |x| lo + x)];
                // Names are checked in this section alone. Elsewhere they are
                // legitimate: `Claude Code URL Handler` is a real application and
                // the apps list is right to name it -- which is what caught the
                // first version of this test.
                for (_, known_as, ..) in &here {
                    assert!(
                        !section.contains(known_as),
                        "handing a job over names {known_as}"
                    );
                }
            }
        }
    }

    #[test]
    fn prompt_names_the_apps_that_actually_exist() {
        // Guessing at an app that is not installed is the failure this prevents.
        let p = prompt(&ask("open something", &[], false));
        assert!(p.contains("Applications this machine can launch"));

        // Only against the real macOS lookup. Under `portable` this machine runs
        // the code written for everywhere else, which looks where Linux keeps its
        // applications and correctly finds nothing on a Mac -- a combination that
        // exists only in this test run.
        #[cfg(not(feature = "portable"))]
        assert!(
            p.contains("Safari"),
            "a Mac always has Safari; got a short list?"
        );
    }

    #[test]
    fn a_stall_is_stated_only_when_it_happened() {
        assert!(prompt(&ask("x", &[], true)).contains("has not changed"));
        assert!(!prompt(&ask("x", &[], false)).contains("has not changed"));
    }
}
