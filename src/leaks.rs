use topcoat::{
    Result,
    context::Cx,
    router::page,
    runtime::signal,
    view::{View, view},
};

use crate::tasks::{Column, Task, db};

#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let tasks = Task::all().exec(&mut db(cx)).await?;

    // 1. An early `return` inside `if`, in a click handler.
    let blocked = tasks.iter().any(|task| task.blocked);
    let attempts = signal(cx, || 0usize);
    let fixed_attempts = signal(cx, || 0usize);

    // 2. A captured non-finite float. Review starts empty, so this is 0.0 / 0.0.
    let review: Vec<&Task> = tasks
        .iter()
        .filter(|task| task.column == Column::Review)
        .collect();
    let blocked_share =
        review.iter().filter(|task| task.blocked).count() as f64 / review.len() as f64;
    let show_share = signal(cx, || true);

    Ok(view! {
        <h1>"Where the boundary leaks"</h1>

        <h2>"1. Early return inside if"</h2>
        <p>
            "The board has a blocked task: "
            (blocked)
        </p>
        <button
            id="early-return"
            @click=$(|_e| {
                if blocked {
                    return;
                }
                attempts.increment();
            })
        >
            "Attempt move (early return)"
        </button>
        " attempts recorded: "
        <output id="attempts">$(attempts.get())</output>
        <br>
        <button
            id="if-else"
            @click=$(|_e| {
                if !blocked {
                    fixed_attempts.increment();
                }
            })
        >
            "Attempt move (if without return)"
        </button>
        " attempts recorded: "
        <output id="fixed-attempts">$(fixed_attempts.get())</output>

        <h2>"2. Captured NaN"</h2>
        <button id="toggle-share" @click=$(|_e| show_share.toggle())>"Toggle"</button>
        " blocked share in Review: "
        <output id="share">
            $(if show_share.get() { blocked_share } else { -1.0 })
        </output>
    })
}
