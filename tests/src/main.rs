mod test_utils;

use std::time::Duration;
use termlens::Terminal;
use test_utils::{
    dismiss_startup_dialog, expect_full_text_not_to_contain, expect_full_text_to_contain,
    expect_tab_bar_not_to_contain, expect_tab_bar_to_contain, expect_view_not_to_contain,
    expect_view_to_contain, press_tab_mode_key, spawn_zsh, unique_suffix, wait_for_plugin_load,
    write_line, D10, D2, D5, D8,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

type TestFn = fn() -> Result<()>;

fn main() {
    let tests: [(&str, TestFn); 5] = [
        ("renames tab on navigation", main_tab_rename),
        ("prefixes tab titles when a pane is waiting", pane_status),
        ("renames tab when closing a pane", close_pane),
        ("handles auto tab names after closing tabs", stable_id),
        ("names tabs by initial working directory", seed_tab_names),
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

fn open_tab_in_dir(t: &mut Terminal, dir: &str) -> Result<()> {
    // zellij 0.44 ignores --cwd unless an initial command is supplied.
    write_line(
        t,
        &format!("zellij action new-tab --cwd /home/alice/{dir} -- /bin/zsh"),
    )?;
    expect_view_to_contain(t, &format!("~/{dir} $"), D5)?;
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

fn cd_into_dir(t: &mut Terminal, dir: &str) -> Result<()> {
    write_line(t, &format!("cd ~/{dir}"))?;
    expect_view_to_contain(t, &format!("~/{dir} $"), D5)?;
    Ok(())
}

fn seed_tab_names() -> Result<()> {
    let mut t = spawn_zsh()?;
    let session = format!("seed-tab-names-session-{}", unique_suffix());
    let second = "second";

    expect_full_text_to_contain(&mut t, "Using config /home/alice/.zshrc", D5)?;

    write_line(&mut t, "cd")?;
    expect_view_to_contain(&mut t, "~ $", D5)?;

    write_line(&mut t, &format!("zellij attach -c {session}"))?;
    expect_full_text_to_contain(&mut t, "Pane #1", D10)?;
    expect_full_text_to_contain(&mut t, "Using config /home/alice/.zshrc", D5)?;
    expect_view_to_contain(&mut t, "~ $", D5)?;

    dismiss_startup_dialog(&mut t)?;
    wait_for_plugin_load(&mut t)?;

    write_line(&mut t, &format!("mkdir -p \"{second}\""))?;

    // A pane that never changes directory still names its tab from its
    // initial working directory.
    expect_tab_bar(&mut t, &session, &["~".to_string()], D8)?;

    // A new tab opened in a custom directory is named from that directory,
    // even though its pane keeps the working directory it was created with.
    open_tab_in_dir(&mut t, second)?;
    expect_tab_bar(
        &mut t,
        &session,
        &["~".to_string(), format!("~/{second}")],
        D8,
    )?;

    // Closing the tab removes its name from the tab bar.
    close_focused_tab(&mut t)?;
    expect_tab_bar(&mut t, &session, &["~".to_string()], D8)?;
    expect_view_not_to_contain(&mut t, &format!("~/{second}"), D2)?;
    Ok(())
}

fn stable_id() -> Result<()> {
    let mut t = spawn_zsh()?;
    let session = format!("stable-id-session-{}", unique_suffix());

    let second = "second";
    let third = "third";
    let fourth = "fourth";
    let fifth = "fifth";
    let renamed = "renamed";

    expect_full_text_to_contain(&mut t, "Using config /home/alice/.zshrc", D5)?;

    write_line(&mut t, "cd")?;
    expect_view_to_contain(&mut t, "~ $", D5)?;

    write_line(&mut t, &format!("zellij attach -c {session}"))?;
    expect_full_text_to_contain(&mut t, "Pane #1", D10)?;
    expect_full_text_to_contain(&mut t, "Using config /home/alice/.zshrc", D5)?;
    expect_view_to_contain(&mut t, "~ $", D5)?;

    dismiss_startup_dialog(&mut t)?;
    wait_for_plugin_load(&mut t)?;

    write_line(
        &mut t,
        &format!("mkdir -p \"{second}\" \"{third}\" \"{fourth}\" \"{fifth}\""),
    )?;

    // Each new tab starts in its own directory, so the plugin seeds a
    // unique tab name for it.
    for dir in [second, third, fourth, fifth] {
        open_tab_in_dir(&mut t, dir)?;
    }
    expect_tab_bar_to_contain(&mut t, &format!("~/{fifth}"), D8)?;

    // Deleting the focused (fifth) tab leaves the fourth tab focused, and
    // its seeded name intact.
    close_focused_tab(&mut t)?;
    expect_tab_bar_to_contain(&mut t, &format!("~/{fourth}"), D8)?;

    // Deleting the focused (fourth) tab leaves the third tab focused and named.
    close_focused_tab(&mut t)?;
    expect_tab_bar_to_contain(&mut t, &format!("~/{third}"), D8)?;

    // The surviving ~/third tab is now the visible third tab. Renaming it
    // must not affect the ~/second tab next to it.
    go_to_tab(&mut t, 3)?;
    write_line(&mut t, &format!("mkdir ~/{renamed}"))?;
    cd_into_dir(&mut t, renamed)?;
    expect_tab_bar_to_contain(&mut t, &format!("~/{renamed}"), D8)?;
    expect_tab_bar_not_to_contain(&mut t, &format!("~/{third}"), D2)?;

    // The second tab kept its own seeded name.
    go_to_tab(&mut t, 2)?;
    expect_tab_bar_to_contain(&mut t, &format!("~/{second}"), D8)?;
    Ok(())
}
