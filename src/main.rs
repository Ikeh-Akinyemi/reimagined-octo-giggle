mod leaks;
mod tasks;

use toasty::Db;
use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    context::{Cx, app_context},
    router::{RouterBuilderDiscoverExt, Slot, layout, module_router, page},
    runtime::{Event, RouterBuilderRuntimeExt, connected, procedure, shard, signal},
    view::{View, component, emit, live, view},
};

use crate::tasks::{BoardEvents, Column, Task, db, seed};

#[tokio::main]
async fn main() {
    // In-memory SQLite keeps the demo self-contained. The board resets on restart.
    let mut db = Db::builder()
        .models(toasty::models!(crate::*))
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db.push_schema().await.unwrap();
    seed(&mut db).await.unwrap();

    topcoat::start(
        module_router!()
            .app_context(db)
            .app_context(BoardEvents::default())
            .assets(AssetBundle::load().unwrap())
            .discover()
            .runtime()
            .build(),
    )
    .await
    .unwrap();
}

#[layout]
async fn root(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"Team board"</title>
                <meta name="viewport" content="width=device-width, initial-scale=1">
                topcoat::dev::script()
                topcoat::runtime::script()
                <style>(STYLES)</style>
            </head>
            <body>(slot)</body>
        </html>
    })
}

#[page]
async fn home(cx: &Cx) -> Result<impl View> {
    // Browser state. Changing either one never contacts the server by itself.
    let hide_done = signal(cx, || false);
    let query = signal(cx, String::new);

    Ok(view! {
        <header>
            <h1>"Team board"</h1>
            <button @click=$(|_e| hide_done.toggle())>
                $(if hide_done.get() { "Show done" } else { "Hide done" })
            </button>
            <input
                type="search"
                placeholder="Search tasks"
                :value=$(query.get())
                @input=$(|e: Event| query.set(e.target.value))
            >
        </header>

        search_results(query: $(query.get()))

        <main :class=$(if hide_done.get() { "board hide-done" } else { "board" })>
            board()
        </main>
    })
}

// Re-renders on the server whenever `query` changes in the browser.
#[shard]
async fn search_results(cx: &Cx, query: String) -> Result<impl View> {
    // The query is user input, so it is trimmed and capped before use.
    let query: String = query.trim().chars().take(100).collect();

    let matches = if query.is_empty() {
        Vec::new()
    } else {
        Task::filter(
            Task::fields()
                .title()
                .like_with_escape(contains_pattern(&query), '\\'),
        )
        .order_by(Task::fields().id().asc())
        .exec(&mut db(cx))
        .await?
    };

    Ok(view! {
        <section class="search">
            if !query.is_empty() {
                <p>
                    (matches.len())
                    " matching "
                    (if matches.len() == 1 { "task" } else { "tasks" })
                </p>
                <ul>
                    for task in &matches {
                        <li>
                            (&task.title)
                            " · "
                            (task.column.label())
                        </li>
                    }
                </ul>
            }
        </section>
    })
}

// Streams a fresh board to every open tab whenever a task moves.
#[shard]
async fn board(cx: &Cx) -> Result<impl View> {
    Ok(live! {
        let mut changed = app_context::<BoardEvents>(cx).subscribe();
        loop {
            let tasks = Task::all()
                .order_by(Task::fields().id().asc())
                .exec(&mut db(cx))
                .await?;

            let token = emit! {
                for column in Column::ALL {
                    column_view(column: column, tasks: &tasks)
                }
            }?;

            if !connected(cx) {
                break Ok(token);
            }
            changed.recv().await.ok();
        }
    })
}

#[component]
async fn column_view(column: Column, tasks: &[Task]) -> Result<impl View> {
    let tasks: Vec<&Task> = tasks.iter().filter(|task| task.column == column).collect();

    Ok(view! {
        <section class=(format!("column column-{}", column.slug()))>
            <h2>
                (column.label())
                " "
                <span>(tasks.len())</span>
            </h2>
            for task in tasks {
                task_card(task: task)
            }
        </section>
    })
}

#[component]
async fn task_card(task: &Task) -> Result<impl View> {
    let id = task.id;
    let back = task
        .column
        .previous()
        .map(Column::slug)
        .unwrap_or_default()
        .to_owned();
    let forward = task
        .column
        .next()
        .map(Column::slug)
        .unwrap_or_default()
        .to_owned();

    Ok(view! {
        <article id=(format!("task-{id}")) class="card">
            <p>(&task.title)</p>
            <small>
                (&task.assignee)
                if task.blocked {
                    " · blocked"
                }
            </small>
            <div class="moves">
                if !back.is_empty() {
                    <button
                        disabled=(task.blocked)
                        @click=$(async |_e| {
                            move_task(id, back.to_owned()).await;
                        })
                    >
                        "←"
                    </button>
                }
                if !forward.is_empty() {
                    <button
                        disabled=(task.blocked)
                        @click=$(async |_e| {
                            move_task(id, forward.to_owned()).await;
                        })
                    >
                        "→"
                    </button>
                }
            </div>
        </article>
    })
}

// Anyone can POST to this endpoint, so every rule is enforced here.
#[procedure]
async fn move_task(cx: &Cx, id: u64, to: String) -> Result<bool> {
    let Some(to) = Column::from_slug(&to) else {
        return Ok(false);
    };

    let mut db = db(cx);
    let Some(mut task) = Task::filter_by_id(id).first().exec(&mut db).await? else {
        return Ok(false);
    };
    let adjacent = task.column.next() == Some(to) || task.column.previous() == Some(to);
    if task.blocked || !adjacent {
        return Ok(false);
    }

    toasty::update!(task { column: to }).exec(&mut db).await?;
    app_context::<BoardEvents>(cx).notify();
    Ok(true)
}

/// Builds a SQL LIKE pattern that matches `text` literally anywhere in a value.
fn contains_pattern(text: &str) -> String {
    let mut pattern = String::from("%");
    for char in text.chars() {
        if matches!(char, '%' | '_' | '\\') {
            pattern.push('\\');
        }
        pattern.push(char);
    }
    pattern.push('%');
    pattern
}

const STYLES: &str = "
body { font: 15px/1.4 system-ui, sans-serif; margin: 0 auto; max-width: 1100px; padding: 16px; }
header { display: flex; gap: 12px; align-items: center; flex-wrap: wrap; }
header h1 { margin: 0 auto 0 0; font-size: 22px; }
input[type=search] { padding: 6px 10px; min-width: 220px; }
.search ul { margin: 4px 0 0; padding-left: 18px; }
.board { display: grid; grid-template-columns: repeat(4, 1fr); gap: 12px; margin-top: 16px; }
.board.hide-done .column-done { display: none; }
.column { background: #f3f4f6; border-radius: 8px; padding: 10px; min-height: 120px; }
.column h2 { font-size: 14px; margin: 0 0 8px; }
.column h2 span { color: #6b7280; font-weight: normal; }
.card { background: #fff; border: 1px solid #e5e7eb; border-radius: 6px; padding: 8px; margin-bottom: 8px; }
.card p { margin: 0 0 4px; }
.card small { color: #6b7280; }
.moves { display: flex; gap: 6px; margin-top: 6px; }
";
