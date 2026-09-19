mod test_utils;

use std::time::Duration;
use termlens::Terminal;
use test_utils::{
    dismiss_startup_dialog, expect_full_text_not_to_contain, expect_full_text_to_contain,
    expect_tab_title, expect_view_not_to_contain, expect_view_to_contain, press_tab_mode_key,
    spawn_zsh, unique_suffix, wait_for_plugin_load, write_line, D10, D2, D5, D8,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

type TestFn = fn() -> Result<()>;

fn main() {
    let tests: [(&str, TestFn); 4] = [
        ("renames tab on navigation", main_tab_rename),
        ("prefixes tab titles when a pane is waiting", pane_status),
        ("renames tab when closing a pane", close_pane),
        ("handles auto tab names after closing tabs", stable_id),
    ];

    let mut failures = 0;
    for (name, run) in tests {
        print!("[RUN ] {name} ... ");
        match run() {
            Ok(()) => println!("ok"),
            Err(e) => {
                failures += 1;
                println!("FAILED");
                eprintln!("{e:#}");
            }
        }
    }

    if failures > 0 {
        eprintln!("\n{failures} test(s) failed");
        std::process::exit(1);
    }
    println!("\nAll tests passed");
}

fn main_tab_rename() -> Result<()> {
    let mut t = spawn_zsh()?;
    let session = "example-session";

    expect_full_text_to_contain(&mut t, "Using config /home/alice/.zshrc", D5)?;

    write_line(&mut t, "whoami")?;
    expect_full_text_to_contain(&mut t, "alice", D5)?;

    write_line(&mut t, "echo $HOME")?;
    expect_full_text_to_contain(&mut t, "/home/alice", D5)?;

    write_line(&mut t, "cd")?;
    write_line(&mut t, "mkdir test1")?;
    write_line(&mut t, "mkdir test2")?;
    write_line(&mut t, "ls")?;

    write_line(&mut t, &format!("zellij attach -c {session}"))?;

    expect_full_text_to_contain(&mut t, "Pane #1", D5)?;
    expect_full_text_to_contain(&mut t, &format!("Zellij ({session})  Tab #1"), D5)?;
    expect_full_text_to_contain(&mut t, "Using config /home/alice/.zshrc", D5)?;
    expect_full_text_to_contain(&mut t, "~ $", D5)?;

    dismiss_startup_dialog(&mut t)?;
    wait_for_plugin_load(&mut t)?;

    write_line(&mut t, "cd test1")?;
    expect_full_text_to_contain(&mut t, "~/test1 $", D5)?;
    expect_full_text_to_contain(&mut t, "Pane #1", D5)?;
    expect_full_text_not_to_contain(&mut t, &format!("Zellij ({session})  Tab #1"), D5)?;
    expect_full_text_to_contain(&mut t, &format!("Zellij ({session})  ~/test1"), D5)?;

    write_line(&mut t, "cd ~")?;
    expect_full_text_to_contain(&mut t, "Pane #1", D5)?;
    expect_full_text_not_to_contain(&mut t, &format!("Zellij ({session})  ~/test1"), D5)?;
    expect_full_text_to_contain(&mut t, &format!("Zellij ({session})  ~"), D5)?;

    write_line(&mut t, "mkdir -p git-project/src/nested")?;
    write_line(&mut t, "cd git-project")?;
    write_line(&mut t, "git init -q")?;
    write_line(&mut t, "cd src/nested")?;
    expect_full_text_to_contain(&mut t, "~/git-project/src/nested $", D5)?;
    expect_view_to_contain(
        &mut t,
        &format!("Zellij ({session})  git-project/src/nested"),
        D5,
    )?;
    expect_view_not_to_contain(
        &mut t,
        &format!("Zellij ({session})  ~/git-project/src/nested"),
        D5,
    )?;

    write_line(&mut t, "cd ../..")?;
    expect_view_to_contain(&mut t, &format!("Zellij ({session})  git-project"), D5)?;

    write_line(&mut t, "git config user.email test@example.com")?;
    write_line(&mut t, "git config user.name 'Tabula Test'")?;
    write_line(&mut t, "touch src/nested/.keep")?;
    write_line(&mut t, "git add .")?;
    write_line(&mut t, "git commit -qm init")?;
    write_line(
        &mut t,
        "git worktree add -q ../git-project-worktree -b worktree-test",
    )?;
    write_line(&mut t, "cd ../git-project-worktree/src/nested")?;
    expect_full_text_to_contain(&mut t, "~/git-project-worktree/src/nested $", D5)?;
    expect_view_to_contain(
        &mut t,
        &format!("Zellij ({session})  git-project/src/nested (\u{1F332} git-projec...)"),
        D5,
    )?;
    expect_view_not_to_contain(
        &mut t,
        &format!("Zellij ({session})  git-project-worktree/src/nested"),
        D5,
    )?;

    write_line(&mut t, "cd ../..")?;
    expect_view_to_contain(
        &mut t,
        &format!("Zellij ({session})  git-project (\u{1F332} git-projec...)"),
        D5,
    )?;
    Ok(())
}

fn pane_status() -> Result<()> {
    let mut t = spawn_zsh()?;
    let u = unique_suffix();
    let session = format!("pane-status-session-{u}");
    let target_dir = format!("pane-status-dir-{u}");

    expect_full_text_to_contain(&mut t, "Using config /home/alice/.zshrc", D5)?;

    write_line(&mut t, "cd")?;
    expect_view_to_contain(&mut t, "~ $", D5)?;

    write_line(&mut t, &format!("zellij attach -c {session}"))?;
    expect_full_text_to_contain(&mut t, "Pane #1", D10)?;
    expect_full_text_to_contain(&mut t, "Using config /home/alice/.zshrc", D5)?;
    expect_view_to_contain(&mut t, "~ $", D5)?;

    dismiss_startup_dialog(&mut t)?;
    wait_for_plugin_load(&mut t)?;

    write_line(&mut t, &format!("mkdir {target_dir}"))?;
    write_line(&mut t, &format!("cd {target_dir}"))?;
    expect_view_to_contain(&mut t, &format!("~/{target_dir} $"), D10)?;
    expect_view_to_contain(&mut t, &format!("Zellij ({session})  ~/{target_dir}"), D5)?;

    write_line(
        &mut t,
        "zellij pipe --name tabula -- \"status '$ZELLIJ_PANE_ID' 'waiting'\"",
    )?;
    expect_view_to_contain(
        &mut t,
        &format!("Zellij ({session})  \u{23F3}~/{target_dir}"),
        D5,
    )?;

    write_line(
        &mut t,
        "zellij pipe --name tabula -- \"status '$ZELLIJ_PANE_ID' 'none'\"",
    )?;
    expect_view_to_contain(&mut t, &format!("Zellij ({session})  ~/{target_dir}"), D5)?;
    expect_view_not_to_contain(
        &mut t,
        &format!("Zellij ({session})  \u{23F3}~/{target_dir}"),
        D2,
    )?;
    Ok(())
}

fn close_pane() -> Result<()> {
    let mut t = spawn_zsh()?;
    let session = format!("close-pane-session-{}", unique_suffix());

    expect_full_text_to_contain(&mut t, "Using config /home/alice/.zshrc", D5)?;

    write_line(&mut t, "cd")?;
    expect_view_to_contain(&mut t, "~ $", D5)?;

    write_line(&mut t, &format!("zellij attach -c {session}"))?;
    expect_full_text_to_contain(&mut t, "Pane #1", D10)?;
    expect_full_text_to_contain(&mut t, "Using config /home/alice/.zshrc", D5)?;
    expect_view_to_contain(&mut t, "~ $", D5)?;

    dismiss_startup_dialog(&mut t)?;
    wait_for_plugin_load(&mut t)?;

    // Create shared directory structure.
    write_line(&mut t, "mkdir -p shared/abc shared/xyz")?;
    expect_view_to_contain(&mut t, "~ $", D5)?;

    // Navigate pane 1 to shared/abc — tab should be named ~/shared/abc.
    write_line(&mut t, "cd shared/abc")?;
    expect_view_to_contain(&mut t, "~/shared/abc $", D5)?;
    expect_view_to_contain(&mut t, &format!("Zellij ({session})  ~/shared/abc"), D5)?;

    // Create a second pane.
    write_line(&mut t, "zellij action new-pane")?;
    expect_view_to_contain(&mut t, "Pane #2", D5)?;
    expect_view_to_contain(&mut t, "~ $", D5)?;

    // Navigate pane 2 to shared/xyz — now both working dirs are known and
    // the tab should show the combined name.
    write_line(&mut t, "cd ~/shared/xyz")?;
    expect_view_to_contain(&mut t, "~/shared/xyz $", D5)?;
    expect_view_to_contain(
        &mut t,
        &format!("Zellij ({session})  ~/shared/* (2 panes)"),
        D5,
    )?;

    // Close pane 2 by exiting its shell.
    write_line(&mut t, "exit")?;
    expect_view_not_to_contain(&mut t, "Pane #2", D5)?;
    expect_view_to_contain(&mut t, "~/shared/abc $", D5)?;
    expect_view_to_contain(&mut t, &format!("Zellij ({session})  ~/shared/abc"), D5)?;
    expect_view_not_to_contain(
        &mut t,
        &format!("Zellij ({session})  ~/shared/* (2 panes)"),
        D2,
    )?;
    Ok(())
}

fn tab_bar_text(session: &str, titles: &[String]) -> String {
    format!("Zellij ({session})  {}", titles.join("  "))
}

fn expect_tab_bar(
    t: &mut Terminal,
    session: &str,
    titles: &[String],
    timeout: Duration,
) -> Result<()> {
    expect_view_to_contain(t, &tab_bar_text(session, titles), timeout)?;
    Ok(())
}

fn new_tab(t: &mut Terminal, tab_number: u32) -> Result<()> {
    let label = format!("Tab #{tab_number}");
    press_tab_mode_key(t, 'n', Some(&label), D5)?;
    Ok(())
}

fn go_to_tab(t: &mut Terminal, tab_number: u32) -> Result<()> {
    let digit = char::from_digit(tab_number, 10).expect("single-digit tab index");
    press_tab_mode_key(t, digit, None, D5)?;
    Ok(())
}

fn close_focused_tab(t: &mut Terminal) -> Result<()> {
    press_tab_mode_key(t, 'x', None, D5)?;
    Ok(())
}

fn cd_into_tab_name_dir(t: &mut Terminal, tab_name: &str) -> Result<()> {
    write_line(t, &format!("cd ~/{tab_name}"))?;
    expect_view_to_contain(t, &format!("~/{tab_name} $"), D5)?;
    Ok(())
}

fn stable_id() -> Result<()> {
    let mut t = spawn_zsh()?;
    let session = format!("stable-id-session-{}", unique_suffix());

    let second = "second";
    let third = "third";
    let fourth = "fourth";
    let fifth = "fifth";

    expect_full_text_to_contain(&mut t, "Using config /home/alice/.zshrc", D5)?;

    write_line(&mut t, "cd")?;
    expect_view_to_contain(&mut t, "~ $", D5)?;

    write_line(&mut t, &format!("zellij attach -c {session}"))?;
    expect_full_text_to_contain(&mut t, "Pane #1", D10)?;
    expect_full_text_to_contain(&mut t, "Using config /home/alice/.zshrc", D5)?;
    expect_view_to_contain(&mut t, "~ $", D5)?;

    write_line(
        &mut t,
        &format!("mkdir -p \"{second}\" \"{third}\" \"{fourth}\" \"{fifth}\""),
    )?;
    expect_tab_title(&mut t, "Tab #1", &session, D8)?;

    dismiss_startup_dialog(&mut t)?;

    for tab_count in 1..6 {
        new_tab(&mut t, tab_count + 1)?;
    }
    expect_view_to_contain(&mut t, "\u{2190} +3  Tab #4  Tab #5  Tab #6", D8)?;

    // Name tab #2 by entering ~/second.
    go_to_tab(&mut t, 2)?;
    cd_into_tab_name_dir(&mut t, second)?;
    expect_view_to_contain(&mut t, &format!("Tab #1  ~/{second}  Tab #3"), D8)?;
    expect_view_to_contain(&mut t, "+3", D8)?;

    // Delete the visible third tab twice.
    go_to_tab(&mut t, 3)?;
    close_focused_tab(&mut t)?;
    expect_view_to_contain(&mut t, &format!("Tab #1  ~/{second}  Tab #4"), D8)?;
    expect_view_to_contain(&mut t, "+2", D8)?;
    expect_view_not_to_contain(&mut t, "Tab #3", D2)?;

    go_to_tab(&mut t, 3)?;
    close_focused_tab(&mut t)?;
    expect_tab_bar(
        &mut t,
        &session,
        &[
            "Tab #1".to_string(),
            format!("~/{second}"),
            "Tab #5".to_string(),
            "Tab #6".to_string(),
        ],
        D8,
    )?;
    expect_view_not_to_contain(&mut t, "Tab #4", D2)?;

    // The old fifth tab is now visible tab #3, so entering ~/third should name it.
    go_to_tab(&mut t, 3)?;
    cd_into_tab_name_dir(&mut t, third)?;
    expect_tab_bar(
        &mut t,
        &session,
        &[
            "Tab #1".to_string(),
            format!("~/{second}"),
            format!("~/{third}"),
            "Tab #6".to_string(),
        ],
        D8,
    )?;

    // The old sixth tab is now visible tab #4, so entering ~/fourth should name it.
    go_to_tab(&mut t, 4)?;
    cd_into_tab_name_dir(&mut t, fourth)?;
    expect_tab_bar(
        &mut t,
        &session,
        &[
            "Tab #1".to_string(),
            format!("~/{second}"),
            format!("~/{third}"),
            format!("~/{fourth}"),
        ],
        D8,
    )?;
    expect_view_not_to_contain(&mut t, "Tab #5", D2)?;
    expect_view_not_to_contain(&mut t, "Tab #6", D2)?;

    // Renaming the surviving fourth visible tab again should not affect tab #3.
    go_to_tab(&mut t, 4)?;
    cd_into_tab_name_dir(&mut t, fifth)?;
    expect_tab_bar(
        &mut t,
        &session,
        &[
            "Tab #1".to_string(),
            format!("~/{second}"),
            format!("~/{third}"),
            format!("~/{fifth}"),
        ],
        D8,
    )?;
    Ok(())
}
