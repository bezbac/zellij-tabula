use std::time::{Duration, SystemTime, UNIX_EPOCH};
use termlens::{Error, Key, Terminal};

pub const D2: Duration = Duration::from_secs(2);
pub const D5: Duration = Duration::from_secs(5);
pub const D8: Duration = Duration::from_secs(8);
pub const D10: Duration = Duration::from_secs(10);

pub fn spawn_zsh() -> Result<Terminal, Error> {
    Terminal::builder()
        .size(80, 30)
        .timeout(Duration::from_secs(15))
        .current_dir("/home/alice")
        .cell_size(8, 16)
        .spawn("/bin/zsh")
}

// Zellij shows an "About Zellij" first-run tips dialog that swallows keystrokes.
// Wait a beat for it to appear (it can render a moment after attach),
// then dismiss it with ESC when present.
pub fn dismiss_startup_dialog(t: &mut Terminal) -> Result<(), Error> {
    let _ = t.wait_until_for(|s| s.contains("About Zellij"), Duration::from_secs(1));
    if t.screen().contains("About Zellij") {
        t.send(Key::Esc)?;
        expect_view_not_to_contain(t, "About Zellij", D5)?;
    }
    Ok(())
}

pub fn unique_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

pub fn wait_for(ms: u64) {
    std::thread::sleep(Duration::from_millis(ms));
}

pub fn write_line(t: &mut Terminal, value: &str) -> Result<(), Error> {
    t.send_str(value)?;
    t.send(Key::Enter)?;
    Ok(())
}

pub fn expect_view_to_contain(
    t: &mut Terminal,
    expected: &str,
    timeout: Duration,
) -> Result<(), Error> {
    t.wait_until_for(|s| s.contains(expected), timeout)?;
    Ok(())
}

pub fn expect_view_not_to_contain(
    t: &mut Terminal,
    unexpected: &str,
    timeout: Duration,
) -> Result<(), Error> {
    t.wait_until_for(|s| !s.contains(unexpected), timeout)?;
    Ok(())
}

pub fn expect_full_text_to_contain(
    t: &mut Terminal,
    expected: &str,
    timeout: Duration,
) -> Result<(), Error> {
    t.wait_until_for(|s| s.full_text().contains(expected), timeout)?;
    Ok(())
}

pub fn expect_full_text_not_to_contain(
    t: &mut Terminal,
    unexpected: &str,
    timeout: Duration,
) -> Result<(), Error> {
    t.wait_until_for(|s| !s.full_text().contains(unexpected), timeout)?;
    Ok(())
}

// Ctrl-T to enter tab mode, then the action key.
pub fn press_tab_mode_key(
    t: &mut Terminal,
    key: char,
    expected: Option<&str>,
    timeout: Duration,
) -> Result<(), Error> {
    t.send_str("\u{14}")?;
    // Let zellij finish entering tab mode before the action key lands;
    // otherwise the two bytes can coalesce and the key gets dropped.
    wait_for(30);
    t.send(Key::Char(key))?;
    match expected {
        Some(text) => expect_view_to_contain(t, text, timeout)?,
        None => wait_for(50),
    }
    Ok(())
}

// zellij only re-sends TabUpdate on a layout change,
// so force one by creating and immediately closing a new tab.
pub fn wait_for_plugin_load(t: &mut Terminal) -> Result<(), Error> {
    press_tab_mode_key(t, 'n', Some("Tab #2"), D5)?;
    press_tab_mode_key(t, 'x', None, D5)?;
    expect_view_not_to_contain(t, "Tab #2", Duration::from_secs(2))?;
    Ok(())
}

pub fn expect_tab_title(
    t: &mut Terminal,
    expected: &str,
    session: &str,
    timeout: Duration,
) -> Result<(), Error> {
    expect_view_to_contain(t, &format!("Zellij ({session})  {expected}"), timeout)
}
