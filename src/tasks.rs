use toasty::Db;
use tokio::sync::broadcast;
use topcoat::context::{Cx, app_context};

#[derive(Debug, Clone, Copy, PartialEq, Eq, toasty::Embed)]
pub enum Column {
    Todo,
    Doing,
    Review,
    Done,
}

impl Column {
    pub const ALL: [Self; 4] = [Self::Todo, Self::Doing, Self::Review, Self::Done];

    pub fn label(self) -> &'static str {
        match self {
            Self::Todo => "To do",
            Self::Doing => "Doing",
            Self::Review => "Review",
            Self::Done => "Done",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Self::Todo => "todo",
            Self::Doing => "doing",
            Self::Review => "review",
            Self::Done => "done",
        }
    }

    // Slugs arrive from the browser, so anything unknown is rejected.
    pub fn from_slug(slug: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|column| column.slug() == slug)
    }

    pub fn next(self) -> Option<Self> {
        match self {
            Self::Todo => Some(Self::Doing),
            Self::Doing => Some(Self::Review),
            Self::Review => Some(Self::Done),
            Self::Done => None,
        }
    }

    pub fn previous(self) -> Option<Self> {
        match self {
            Self::Todo => None,
            Self::Doing => Some(Self::Todo),
            Self::Review => Some(Self::Doing),
            Self::Done => Some(Self::Review),
        }
    }
}

#[derive(Debug, toasty::Model)]
pub struct Task {
    #[key]
    #[auto]
    pub id: u64,

    pub title: String,

    pub assignee: String,

    pub column: Column,

    pub blocked: bool,
}

pub fn db(cx: &Cx) -> Db {
    app_context::<Db>(cx).clone()
}

/// Notifies every connected board when a task changes.
pub struct BoardEvents {
    changed: broadcast::Sender<()>,
}

impl BoardEvents {
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.changed.subscribe()
    }

    pub fn notify(&self) {
        // An error only means no board is connected right now.
        let _ = self.changed.send(());
    }
}

impl Default for BoardEvents {
    fn default() -> Self {
        Self {
            changed: broadcast::channel(16).0,
        }
    }
}

pub async fn seed(db: &mut Db) -> toasty::Result<()> {
    let tasks = [
        ("Write the onboarding guide", "Ada", Column::Todo, false),
        ("Rotate the staging API keys", "Linus", Column::Todo, true),
        ("Add rate limiting to /login", "Grace", Column::Doing, false),
        ("Fix flaky checkout test", "Ada", Column::Doing, false),
        ("Ship the billing export", "Linus", Column::Done, false),
        ("Upgrade to Rust 1.99", "Grace", Column::Done, false),
    ];

    for (title, assignee, column, blocked) in tasks {
        toasty::create!(Task {
            title,
            assignee,
            column,
            blocked,
        })
        .exec(db)
        .await?;
    }

    Ok(())
}
